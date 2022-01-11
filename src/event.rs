use serde::{Deserialize, Serialize};

schemafy::schemafy!("event_package.json");

// Unlike genereated binding, an `enum` rather than a `struct`
include!(concat!(env!("OUT_DIR"), "/event_enum.rs"));

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub data: Vec<AnyTelemetryEventEnum>,
    pub data_header: TelemetryHeaderModel,
}
