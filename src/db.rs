use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef},
    Connection, Result, Statement,
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
            INSERT OR IGNORE INTO consent (id, opted_in_level, version) VALUES (0, NULL, NULL);
        ",
        )?;
        // Migrate here if schema changes are made
        Ok(Self(conn))
    }

    pub fn prepare_queue_insert(&self) -> Result<QueueInsert> {
        self.0
            .prepare("INSERT INTO queued_events (value) VALUES (?)")
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
                "REPLACE INTO consent (id, opted_in_level, version) VALUES (0, ?, ?)",
                [opted_in_level, version],
            )
            .map(|_| ())
    }

    pub fn update_event_types(&self) -> Result<()> {
        // TODO: take argument; when/show should this be initialized? Include default with package,
        // or query server first?

        let mut insert_statement = self
            .0
            .prepare("INSERT into event_types (type, frequency) VALUES (?, ?) ON CONFLICT(type) DO UPDATE SET frequency=excluded.frequency")?;

        let tx = self.0.unchecked_transaction()?;
        for i in TelemetryEventType::iter() {
            let frequency = match i.name() {
                "hw_peripheral_usb_type_a" | "hw_thermal_context" => "trigger",
                _ => "daily",
            };
            insert_statement.execute([i.name(), frequency])?;
        }
        tx.commit()
    }

    pub fn get_state_with_freq(&self, freq: &str) -> Result<Vec<TelemetryEvent>> {
        let mut stmt = self.0.prepare("SELECT state.value from state INNER JOIN event_types USING(type) WHERE event_types.frequency = ?")?;
        let rows = stmt.query_map([freq], |row| row.get(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn replace_state_with_freq(&self, freq: &str, events: &[TelemetryEvent]) -> Result<()> {
        let mut insert_statement = self
            .0
            .prepare("INSERT into state (type, value) VALUES (?, ?)")?;

        let tx = self.0.unchecked_transaction()?;
        self.0.execute("DELETE from state where ROWID IN (SELECT state.ROWID from state INNER JOIN event_types USING(type) WHERE event_types.frequency = ?)", [freq])?;
        for i in events {
            insert_statement.execute(params!(i.type_().name(), i))?;
        }
        tx.commit()
    }

    // Uses `seen` property so `clear_queued` doesn't delete things added after this
    pub fn get_queued(&self) -> Result<Vec<TelemetryEvent>> {
        let tx = self.0.unchecked_transaction()?;
        self.0.execute("UPDATE queued_events SET seen = 1", [])?;
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
