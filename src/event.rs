use serde::{Deserialize, Serialize};

schemafy::schemafy!("UploadEventPackageRequestModel.json");

// Unlike genereated binding, an `enum` rather than a `struct`
include!(concat!(env!("OUT_DIR"), "/event_enum.rs"));

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub data: Vec<AnyTelemetryEventEnum>,
    pub data_header: TelemetryHeaderModel,
}

impl Event {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
