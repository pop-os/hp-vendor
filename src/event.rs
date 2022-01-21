use os_release::OsRelease;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, str::FromStr};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

schemafy::schemafy!("UploadEventPackageRequestModel.json");

// Unlike genereated binding, an `enum` rather than a `struct`
include!(concat!(env!("OUT_DIR"), "/event_enum.rs"));

pub(crate) fn read_file<P: AsRef<Path>, T: FromStr>(path: P) -> Option<T> {
    fs::read_to_string(path)
        .ok()
        .and_then(|x| x.trim().parse().ok())
}

pub(crate) fn unknown() -> String {
    "unknown".to_string()
}

pub fn data_header() -> TelemetryHeaderModel {
    let (os_name, os_version) = match OsRelease::new() {
        Ok(OsRelease { name, version, .. }) => (name, version),
        Err(_) => (unknown(), unknown()),
    };

    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .ok()
        .unwrap_or_else(unknown);

    TelemetryHeaderModel {
        consent: DataCollectionConsent {
            opted_in_level: String::new(), // XXX
            version: String::new(),        // XXX
        },
        data_provider: DataProviderInfo {
            app_name: env!("CARGO_PKG_NAME").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_name,
            os_version,
        },
        ids: DeviceOSIds {
            bios_uuid: String::new(),            // TODO
            device_id: "XXXXXXXXXX".to_string(), // TODO
            os_install_id: String::new(),        // TODO
        },
        timestamp,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub data: Vec<AnyTelemetryEventEnum>,
    pub data_header: TelemetryHeaderModel,
}

impl Event {
    pub fn new(data: Vec<AnyTelemetryEventEnum>) -> Self {
        Self {
            data,
            data_header: data_header(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
