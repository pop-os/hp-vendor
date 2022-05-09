// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef},
    Connection, OptionalExtension, Result, Statement,
};
use std::{collections::HashMap, error::Error, fmt, str};
use time::{Duration, OffsetDateTime};

use crate::{
    config::SamplingFrequency,
    event::{DataCollectionConsent, DataCollectionPurpose, TelemetryEvent, TelemetryEventType},
    frequency::Frequencies,
    util,
};

pub enum State<'a> {
    All,
    Frequency(SamplingFrequency),
    #[allow(dead_code)]
    Type(TelemetryEventType),
    Ids(&'a [i64]),
}

fn migration1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE event_types (
             type TEXT NOT NULL PRIMARY KEY,
             frequency TEXT NOT NULL
         );
         CREATE TABLE state (
             id INTEGER PRIMARY KEY,
             type TEXT NOT NULL,
             value TEXT NOT NULL,
             FOREIGN KEY(type) REFERENCES event_types(type) ON DELETE CASCADE
         );
         CREATE TABLE queued_events (
             id INTEGER PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE consents (
             id INTEGER PRIMARY KEY,
             locale TEXT NOT NULL,
             country TEXT NOT NULL,
             purpose_id TEXT NOT NULL,
             version TEXT NOT NULL,
             sent INTEGER DEFAULT 0 NOT NULL
         );
         CREATE TABLE purposes (
             id INTEGER PRIMARY KEY,
             locale TEXT NOT NULL,
             purpose_id TEXT NOT NULL,
             version TEXT NOT NULL,
             min_version TEXT NOT NULL,
             statement TEXT NOT NULL
         );
         CREATE TABLE properties (
             id INTEGER PRIMARY KEY,
             os_install_id TEXT,
             last_weekly_time INTEGER,
             CHECK (id = 0)
         );",
    )?;
    conn.execute(
        "INSERT into properties (id, os_install_id, last_weekly_time)
         VALUES (0, ?, 0)",
        [uuid::Uuid::new_v4().to_string()],
    )?;
    Ok(())
}

fn migration2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE temps (
             id INTEGER PRIMARY KEY,
             cpu INTEGER NOT NULL,
             ext INTEGER NOT NULL,
             bat INTEGER NOT NULL,
             chg INTEGER NOT NULL,
             on_ac INTEGER NOT NULL,
             charging INTEGER NOT NULL,
             time INTEGER NOT NULL
        );",
    )?;
    Ok(())
}

static MIGRATIONS: &[fn(&Connection) -> Result<()>] = &[migration1, migration2];

pub struct DB(Connection);

impl DB {
    pub fn open() -> Result<Self> {
        let conn = Connection::open("/var/hp-vendor/db.sqlite3")?;

        let tx = conn.unchecked_transaction()?;
        let user_version: usize = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        for migration in &MIGRATIONS[user_version..] {
            migration(&conn)?;
        }
        conn.pragma_update(None, "user_version", MIGRATIONS.len())?;
        tx.commit()?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let db = Self(conn);
        db.init_event_types()?;
        Ok(db)
    }

    pub fn prepare_queue_insert(&self) -> Result<QueueInsert> {
        self.0
            .prepare(
                "INSERT INTO queued_events (value)
                 VALUES (?)",
            )
            .map(QueueInsert)
    }

    // Should be checked before upload, etc.
    pub fn get_consent(&self) -> Result<Option<DataCollectionConsent>> {
        let mut stmt = self
            .0
            .prepare("SELECT locale, country, purpose_id, version, sent from consents")?;
        stmt.query_row([], |row| {
            Ok(DataCollectionConsent {
                locale: row.get(0)?,
                country: row.get(1)?,
                purpose_id: row.get(2)?,
                version: row.get(3)?,
                sent: row.get(4)?,
            })
        })
        .optional()
    }

    pub fn set_consent(&self, consent: Option<&DataCollectionConsent>) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        self.0.execute("DELETE FROM consents", [])?;
        let mut stmt = self.0.prepare(
            "INSERT INTO consents (locale, country, purpose_id, version, sent)
             VALUES (?, ?, ?, ?, ?)",
        )?;
        if let Some(consent) = consent {
            stmt.execute(params![
                &consent.locale,
                &consent.country,
                &consent.purpose_id,
                &consent.version,
                &consent.sent
            ])?;
        }
        tx.commit()
    }

    pub fn get_purposes(&self) -> Result<HashMap<String, DataCollectionPurpose>> {
        let mut stmt = self
            .0
            .prepare("SELECT locale, purpose_id, version, min_version, statement FROM purposes")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                DataCollectionPurpose {
                    purpose_id: row.get(1)?,
                    version: row.get(2)?,
                    min_version: row.get(3)?,
                    statement: row.get(4)?,
                },
            ))
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn set_purposes(&self, purposes: &HashMap<String, DataCollectionPurpose>) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        self.0.execute("DELETE FROM purposes", [])?;
        let mut stmt = self.0.prepare(
            "INSERT INTO purposes (locale, purpose_id, version, min_version, statement)
             VALUES (?, ?, ?, ?, ?)",
        )?;
        for (locale, i) in purposes.iter() {
            stmt.execute(params![
                &locale,
                &i.purpose_id,
                &i.version,
                &i.min_version,
                &i.statement,
            ])?;
        }
        tx.commit()
    }

    pub fn get_os_install_id(&self) -> Result<String> {
        self.0
            .query_row("SELECT os_install_id from properties", [], |row| row.get(0))
    }

    fn get_last_weekly_time(&self) -> Result<OffsetDateTime> {
        let time: Option<i64> =
            self.0
                .query_row("SELECT last_weekly_time from properties", [], |row| {
                    row.get(0)
                })?;
        Ok(OffsetDateTime::from_unix_timestamp(time.unwrap_or(0))
            .unwrap_or(OffsetDateTime::UNIX_EPOCH))
    }

    pub fn last_weekly_time_expired(&self) -> Result<bool> {
        let diff = OffsetDateTime::now_utc() - self.get_last_weekly_time()?;
        Ok(diff.is_negative() || diff > Duration::WEEK)
    }

    pub fn update_last_weekly_time(&self) -> Result<()> {
        let time = OffsetDateTime::now_utc().unix_timestamp();
        self.0
            .execute("UPDATE properties SET last_weekly_time = ?", [time])
            .map(|_| ())
    }

    fn init_event_types(&self) -> Result<()> {
        // Add with default frequency if not already in db
        let mut insert_statement = self.0.prepare(
            "INSERT OR IGNORE into event_types (type, frequency)
             VALUES (?, ?)",
        )?;

        let tx = self.0.unchecked_transaction()?;
        for (type_, frequency) in Frequencies::default().iter() {
            insert_statement.execute(params![type_, frequency])?;
        }
        tx.commit()
    }

    pub fn set_event_frequencies(&self, frequencies: Frequencies) -> Result<()> {
        let mut insert_statement = self.0.prepare(
            "INSERT into event_types (type, frequency)
             VALUES (?, ?)
             ON CONFLICT(type) DO
                 UPDATE SET frequency=excluded.frequency",
        )?;

        let tx = self.0.unchecked_transaction()?;
        for (type_, frequency) in frequencies.iter() {
            insert_statement.execute(params![type_, frequency])?;
        }
        tx.commit()
    }

    pub fn get_event_frequencies(&self) -> Result<Frequencies> {
        let mut stmt = self.0.prepare("SELECT type, frequency from event_types")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        // Ignore invalid types, and use default if somehow it doesn't exist
        Ok(Frequencies::from_iter_or_default(
            rows.filter_map(Result::ok),
        ))
    }

    pub fn get_state(&self, filter: State) -> Result<Vec<TelemetryEvent>> {
        let (mut stmt, params) = match &filter {
            State::All => {
                let stmt = self.0.prepare("SELECT value from state")?;
                (stmt, vec![])
            }
            State::Frequency(freq) => {
                let stmt = self.0.prepare(
                    "SELECT state.value from state
                         INNER JOIN event_types
                         USING(type)
                         WHERE event_types.frequency = ?",
                )?;
                (stmt, vec![freq as &dyn ToSql])
            }
            State::Type(type_) => {
                let stmt = self.0.prepare("SELECT value from state WHERE type = ?")?;
                (stmt, vec![type_ as &dyn ToSql])
            }
            State::Ids(ids) => {
                let mut stmt = self.0.prepare("SELECT value from state WHERE id = ?")?;
                let mut events = Vec::new();
                for id in *ids {
                    if let Some(event) = stmt.query_row([id], |row| row.get(0)).optional()? {
                        events.push(event);
                    }
                }
                return Ok(events);
            }
        };
        let rows = stmt.query_map(&*params, |row| row.get(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn replace_state(&self, filter: State, events: &[TelemetryEvent]) -> Result<Vec<i64>> {
        let mut insert_statement = self.0.prepare(
            "INSERT into state (type, value)
             VALUES (?, ?)",
        )?;

        let tx = self.0.unchecked_transaction()?;
        match filter {
            State::All => {
                self.0.execute("DELETE from state", [])?;
            }
            State::Frequency(freq) => {
                self.0.execute(
                    "DELETE from state
                     WHERE ROWID IN
                         (SELECT state.ROWID from state
                              INNER JOIN event_types
                              USING(type)
                              WHERE event_types.frequency = ?)",
                    [freq],
                )?;
            }
            State::Type(type_) => {
                self.0
                    .execute("DELETE from state WHERE type = ?", [type_])?;
            }
            State::Ids(ids) => {
                let mut stmt = self.0.prepare("DELETE from state WHERE id = ?")?;
                for id in ids {
                    stmt.execute([id])?;
                }
            }
        }
        let mut ids = Vec::new();
        for i in events {
            insert_statement.execute(params!(i.type_().name(), i))?;
            ids.push(self.0.last_insert_rowid());
        }
        tx.commit()?;
        Ok(ids)
    }

    pub fn get_queued(&self) -> Result<(Vec<i64>, Vec<TelemetryEvent>)> {
        // TODO: how to remove anything that doesn't parse?
        // - Shouldn't be needed, but may if certain changes are maid
        let mut stmt = self.0.prepare("SELECT id, value from queued_events")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows
            .filter_map(Result::ok)
            .unzip::<i64, TelemetryEvent, _, _>())
    }

    pub fn remove_queued(&self, ids: &[i64]) -> Result<()> {
        let mut stmt = self.0.prepare("DELETE from queued_events where id = ?")?;
        let tx = self.0.unchecked_transaction()?;
        for id in ids {
            stmt.execute([id])?;
        }
        tx.commit()
    }

    pub fn delete_and_disable(&self) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        self.0.execute_batch(
            "DELETE from state;
             DELETE from queued_events;
             DELETE from consents;
            ",
        )?;
        tx.commit()
    }

    pub fn insert_temps(&self, temps: &util::Temps) -> Result<()> {
        self.0.execute(
            "INSERT INTO temps (cpu, ext, bat, chg, on_ac, charging, time)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                temps.cpu,
                temps.ext,
                temps.bat,
                temps.chg,
                temps.on_ac,
                temps.charging,
                temps.time,
            ],
        )?;
        Ok(())
    }

    pub fn get_temps(&self, limit: bool) -> Result<Vec<util::Temps>> {
        let mut stmt = if limit {
            self.0.prepare(
                "SELECT cpu, ext, bat, chg, on_ac, charging, time FROM temps
                 LIMIT 100
                 SORT BY time",
            )?
        } else {
            self.0.prepare(
                "SELECT cpu, ext, bat, chg, on_ac, charging, time FROM temps
                 SORT BY time",
            )?
        };
        let rows = stmt.query_map([], |row| {
            Ok(util::Temps {
                cpu: row.get(0)?,
                ext: row.get(1)?,
                bat: row.get(2)?,
                chg: row.get(3)?,
                on_ac: row.get(4)?,
                charging: row.get(5)?,
                time: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    // Remove where time less than last
    pub fn remove_temps_before(&self, temps: &util::Temps) -> Result<()> {
        self.0
            .execute("DELETE FROM temps WHERE time <= ?", [temps.time])?;
        Ok(())
    }
}

pub struct QueueInsert<'a>(Statement<'a>);

impl<'a> QueueInsert<'a> {
    pub fn execute(&mut self, event: &TelemetryEvent) -> Result<()> {
        self.0.execute([event]).map(|_| ())
    }
}

fn other_err<E: Error + Send + Sync + 'static>(err: E) -> FromSqlError {
    FromSqlError::Other(Box::new(err))
}

#[derive(Debug)]
struct InvalidEnum(String);

impl fmt::Display for InvalidEnum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}' is not a valid enum variant", self.0)
    }
}

impl Error for InvalidEnum {}

impl ToSql for TelemetryEvent {
    fn to_sql(&self) -> Result<ToSqlOutput> {
        Ok(ToSqlOutput::Owned(Value::Text(
            serde_json::to_string(self).unwrap(),
        )))
    }
}

impl FromSql for TelemetryEvent {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(text) = value {
            serde_json::from_slice(text).map_err(other_err)
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

impl ToSql for TelemetryEventType {
    fn to_sql(&self) -> Result<ToSqlOutput<'static>> {
        self.name().to_sql()
    }
}

impl FromSql for TelemetryEventType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(text) = value {
            let text = str::from_utf8(text).map_err(other_err)?;
            TelemetryEventType::from_str(text)
                .ok_or_else(|| other_err(InvalidEnum(text.to_string())))
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

impl ToSql for SamplingFrequency {
    fn to_sql(&self) -> Result<ToSqlOutput<'static>> {
        self.to_str().to_sql()
    }
}

impl FromSql for SamplingFrequency {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(text) = value {
            let text = str::from_utf8(text).map_err(other_err)?;
            SamplingFrequency::from_str(text)
                .ok_or_else(|| other_err(InvalidEnum(text.to_string())))
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}
