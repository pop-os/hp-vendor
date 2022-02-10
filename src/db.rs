use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef},
    Connection, Error, OptionalExtension, Result, Statement,
};

use crate::event::{DataCollectionConsent, TelemetryEvent, TelemetryEventType};

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
                FOREIGN_KEY(type) NOT NULL REFERENCES event_types(type),
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS queued_events (
                value TEXT NOT NULL,
                seen INTEGER DEFAULT 0 NOT NULL
            );
            );
            CREATE TABLE IF NOT EXISTS consent (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                opted_in_level TEXT,
                version TEXT,
            );
            INSERT OR IGNORE INTO consent (id, opted_in_level, version) VALUES (0, NULL, NULL);
        ",
        )?;
        Ok(Self(conn))
    }

    fn prepare_queue_insert(&self) -> Result<QueueInsert> {
        self.0
            .prepare("INSERT INTO queued_events (value) VALUES (?)")
            .map(QueueInsert)
    }

    // Should be checked before upload, etc.
    fn get_consent(&self) -> Result<Option<DataCollectionConsent>> {
        self.0
            .query_row("SELECT opted_in_level, version from consent", [], |row| {
                Ok(DataCollectionConsent {
                    opted_in_level: row.get(0)?,
                    version: row.get(1)?,
                })
            })
            .optional()
    }

    fn set_consent(&self, consent: &DataCollectionConsent) -> Result<()> {
        self.0
            .execute(
                "REPLACE INTO consent (id, opted_in_level, version) VALUES (0, ?, ?)",
                [&consent.opted_in_level, &consent.version],
            )
            .map(|_| ())
    }

    fn update_event_types(&self) -> Result<()> {
        // TODO: remove from `state` and `queued_events` if removing a `type`
        // TODO: take argument

        let mut insert_statement = self
            .0
            .prepare("INSERT into event_types (type, frequency) VALUES (?, ?)")?;

        self.0.execute("BEGIN", [])?;
        for i in TelemetryEventType::iter() {
            insert_statement.execute([i.name(), "daily"])?;
        }
        self.0.execute("END", [])?;

        Ok(())
    }
}

pub struct QueueInsert<'a>(Statement<'a>);

impl<'a> QueueInsert<'a> {
    fn execute(&mut self, event: &TelemetryEvent) -> Result<()> {
        self.0.execute([event]).map(|_| ())
    }
}

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
            serde_json::from_slice(text).map_err(|err| FromSqlError::Other(Box::new(err)))
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}
