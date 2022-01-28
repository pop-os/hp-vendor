use os_release::OsRelease;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, str::FromStr};
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
pub struct Events {
    pub data: Vec<TelemetryEvent>,
    pub data_header: TelemetryHeaderModel,
}

impl Events {
    pub fn new(data: Vec<TelemetryEvent>) -> Self {
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

impl TelemetryEvent {
    fn diff(&mut self, old: &Self) {
        // Set new state to same as old, before comparison
        if let (Some(mut new_state), Some(v_state)) = (self.state_mut(), old.state()) {
            new_state.set(v_state);
        }
        if self == old {
            match self.state_mut() {
                Some(MutState::Sw(state)) => *state = Swstate::Same,
                Some(MutState::Hw(state)) => *state = Hwstate::Same,
                None => {}
            }
        } else {
            match self.state_mut() {
                Some(MutState::Sw(state)) => *state = Swstate::Updated,
                Some(MutState::Hw(_state)) => {} // XXX ?
                None => {}
            }
        }
        // TODO: how to include only changed fields?
    }
}

pub fn diff(events: &mut Vec<TelemetryEvent>, old_events: &[TelemetryEvent]) {
    // TODO: warn if multiple things have same primary key?

    let mut m1 = HashMap::new();
    for event in events.iter_mut() {
        m1.insert((event.type_(), event.primaries()), event);
    }

    let mut m2 = HashMap::new();
    for event in old_events.iter() {
        m2.insert((event.type_(), event.primaries()), event);
    }

    for (k, new) in m1.iter_mut() {
        if let Some(old) = m2.get(k) {
            new.diff(old);
        } else {
            match new.state_mut() {
                Some(MutState::Sw(state)) => *state = Swstate::Installed,
                Some(MutState::Hw(state)) => *state = Hwstate::Added,
                None => {}
            }
        }
    }

    let mut new_events = Vec::new();
    for (k, old) in m2.iter_mut() {
        if !m1.contains_key(k) {
            let mut new = (**old).clone();
            match new.state_mut() {
                Some(MutState::Sw(state)) => *state = Swstate::Uninstalled,
                Some(MutState::Hw(state)) => *state = Hwstate::Removed,
                None => {}
            }
            // TODO: omit other fields?
            new_events.push(new);
        }
    }
    events.extend(new_events);
}

impl Copy for Swstate {}
impl Copy for Hwstate {}

#[derive(Clone, Copy)]
enum State {
    Sw(Swstate),
    Hw(Hwstate),
}

enum MutState<'a> {
    Sw(&'a mut Swstate),
    Hw(&'a mut Hwstate),
}

impl<'a> MutState<'a> {
    fn set(&mut self, other: State) {
        match (self, other) {
            (Self::Sw(l), State::Sw(r)) => **l = r,
            (Self::Hw(l), State::Hw(r)) => **l = r,
            _ => {}
        }
    }
}
