use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef},
    Connection, Result, Statement,
};
use std::{error::Error, fmt, str};

use crate::{
    event::{DataCollectionConsent, TelemetryEvent, TelemetryEventType},
    frequency::{Frequencies, Frequency},
};

pub enum State {
    All,
    Frequency(Frequency),
    Type(TelemetryEventType),
}

pub struct DB(Connection);

impl DB {
    pub fn open() -> Result<Self> {
        let conn = Connection::open("/var/hp-vendor/db.sqlite3")?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS event_types (
                type TEXT NOT NULL PRIMARY KEY,
                frequency TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS state (
                type TEXT NOT NULL,
                value TEXT NOT NULL,
                FOREIGN KEY(type) REFERENCES event_types(type) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS queued_events (
                value TEXT NOT NULL,
                seen INTEGER DEFAULT 0 NOT NULL
            );
            CREATE TABLE IF NOT EXISTS consent (
                id INTEGER PRIMARY KEY,
                opted_in_level TEXT,
                version TEXT,
                CHECK (id = 0)
            );
            INSERT OR IGNORE INTO consent (id, opted_in_level, version)
            VALUES (0, NULL, NULL);
        ",
        )?;
        // Migrate here if schema changes are made
        Ok(Self(conn))
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
        self.0
            .query_row("SELECT opted_in_level, version from consent", [], |row| {
                Ok((|| {
                    Some(DataCollectionConsent {
                        opted_in_level: row.get(0).ok()?,
                        version: row.get(1).ok()?,
                    })
                })())
            })
    }

    pub fn set_consent(&self, consent: Option<&DataCollectionConsent>) -> Result<()> {
        let opted_in_level = consent.map(|x| &x.opted_in_level);
        let version = consent.map(|x| &x.version);
        self.0
            .execute(
                "REPLACE INTO consent (id, opted_in_level, version)
                 VALUES (0, ?, ?)",
                [opted_in_level, version],
            )
            .map(|_| ())
    }

    pub fn update_event_types(&self) -> Result<()> {
        // TODO: take argument; when/show should this be initialized? Include default with package,
        // or query server first?

        let mut insert_statement = self.0.prepare(
            "INSERT into event_types (type, frequency)
             VALUES (?, ?)
             ON CONFLICT(type) DO
                 UPDATE SET frequency=excluded.frequency",
        )?;

        let tx = self.0.unchecked_transaction()?;
        for (type_, frequency) in Frequencies::default().iter() {
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
        };
        let rows = stmt.query_map(&*params, |row| row.get(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn replace_state(&self, filter: State, events: &[TelemetryEvent]) -> Result<()> {
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
        }
        for i in events {
            insert_statement.execute(params!(i.type_().name(), i))?;
        }
        tx.commit()
    }

    // Uses `seen` property so `clear_queued` doesn't delete things added after this
    pub fn get_queued(&self, mark_seen: bool) -> Result<Vec<TelemetryEvent>> {
        let tx = self.0.unchecked_transaction()?;
        if mark_seen {
            self.0.execute("UPDATE queued_events SET seen = 1", [])?;
        }
        let mut stmt = self.0.prepare("SELECT value from queued_events")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        tx.commit()?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn clear_queued(&self) -> Result<()> {
        self.0
            .execute("DELETE from queued_events where seen = 1", [])
            .map(|_| ())
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

impl ToSql for Frequency {
    fn to_sql(&self) -> Result<ToSqlOutput<'static>> {
        self.to_str().to_sql()
    }
}

impl FromSql for Frequency {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(text) = value {
            let text = str::from_utf8(text).map_err(other_err)?;
            Frequency::from_str(text).ok_or_else(|| other_err(InvalidEnum(text.to_string())))
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}
