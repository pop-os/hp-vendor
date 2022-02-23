use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef},
    Connection, Result, Statement,
};
use std::{error::Error, fmt, str};

use crate::{
    config::SamplingFrequency,
    event::{DataCollectionConsent, TelemetryEvent, TelemetryEventType},
    frequency::Frequencies,
};

pub enum State {
    All,
    Frequency(SamplingFrequency),
    #[allow(dead_code)]
    Type(TelemetryEventType),
}

fn create_tables(conn: &Connection) -> Result<()> {
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
             value TEXT NOT NULL,
             seen INTEGER DEFAULT 0 NOT NULL
         );
         CREATE TABLE consent (
             id INTEGER PRIMARY KEY,
             opted_in_level TEXT,
             version TEXT,
             CHECK (id = 0)
         );
         INSERT INTO consent (id, opted_in_level, version)
         VALUES (0, NULL, NULL);
         CREATE TABLE properties (
             id INTEGER PRIMARY KEY,
             os_install_id TEXT,
             CHECK (id = 0)
         );",
    )?;
    conn.execute(
        "INSERT into properties
         VALUES (0, ?)",
        [uuid::Uuid::new_v4().to_string()],
    )?;
    Ok(())
}

fn migrate_0_to_1(conn: &Connection) -> Result<()> {
    // Add explicit integer primary keys; same as rowid, but if not explicitly
    // defined it can change.
    conn.execute_batch(
        "CREATE TABLE state_new (
             id INTEGER PRIMARY KEY,
             type TEXT NOT NULL,
             value TEXT NOT NULL,
             FOREIGN KEY(type) REFERENCES event_types(type) ON DELETE CASCADE
         );
         CREATE TABLE queued_events_new (
             id INTEGER PRIMARY KEY,
             value TEXT NOT NULL,
             seen INTEGER DEFAULT 0 NOT NULL
         );
         INSERT INTO state_new (type, value)
             SELECT type, value FROM state;
         INSERT INTO queued_events_new (value, seen)
             SELECT value, seen FROM queued_events;
         DROP TABLE state;
         DROP TABLE queued_events;
         ALTER TABLE state_new RENAME TO state;
         ALTER TABLE queued_events_new RENAME TO queued_events;
        ",
    )
}

pub struct DB(Connection);

impl DB {
    pub fn open() -> Result<Self> {
        let conn = Connection::open("/var/hp-vendor/db.sqlite3")?;

        let tx = conn.unchecked_transaction()?;
        let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let empty = !conn
            .prepare("SELECT 1 FROM sqlite_schema where type='table'")?
            .exists([])?;
        if empty {
            create_tables(&conn)?;
        } else if user_version == 0 {
            migrate_0_to_1(&conn)?;
        }
        conn.execute("PRAGMA user_version = 1", [])?;
        tx.commit()?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;

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

    pub fn get_os_install_id(&self) -> Result<String> {
        self.0
            .query_row("SELECT os_install_id from properties", [], |row| row.get(0))
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
