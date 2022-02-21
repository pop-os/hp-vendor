use os_release::OsRelease;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    str::FromStr,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, Time};
use uuid::Uuid;

use crate::util::dmi::{dmi, SystemInfo24};

schemafy::schemafy!("DataUploadRequestModel.json");

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

pub(crate) fn date_time() -> String {
    let now = OffsetDateTime::now_utc();
    let time = now.time();
    // Second precision, instead of nanosecond
    now.replace_time(Time::from_hms(time.hour(), time.minute(), time.second()).unwrap())
        .format(&Rfc3339)
        .unwrap()
}

impl DeviceOSIds {
    pub fn new(os_install_uuid: String) -> anyhow::Result<Self> {
        (|| {
            let dmi = dmi();

            let (i, sys_info) = dmi
                .iter()
                .find_map(|i| Some((i, i.get::<SystemInfo24>()?)))?;
            let device_sku = i.get_str(sys_info.sku).cloned()?;
            let device_bios_uuid = Uuid::from(&sys_info.uuid).to_string();
            let device_sn = i.get_str(sys_info.serial).cloned()?;

            let (i, bb_info) = dmi
                .iter()
                .find_map(|i| Some((i, i.get::<dmi::BaseBoardInfo>()?)))?;
            let device_base_board_id = i.get_str(bb_info.product).cloned()?;

            Some(DeviceOSIds {
                device_sku,
                device_base_board_id,
                device_bios_uuid,
                device_sn,
                os_install_uuid,
            })
        })()
        .ok_or_else(|| anyhow::Error::msg("Unable to get sku, uuid, and serial from BIOS"))
    }
}

fn data_header(consent: DataCollectionConsent, ids: DeviceOSIds) -> TelemetryHeaderModel {
    let (os_name, os_version) = match OsRelease::new() {
        Ok(OsRelease { name, version, .. }) => (name, version),
        Err(_) => (unknown(), unknown()),
    };

    TelemetryHeaderModel {
        consent,
        data_provider: DataProviderInfo {
            app_name: env!("CARGO_PKG_NAME").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_name,
            os_version,
        },
        ids,
        timestamp: date_time(),
    }
}

#[derive(Debug, Serialize)]
pub struct Events<'a> {
    pub data: &'a [TelemetryEvent],
    pub data_header: TelemetryHeaderModel,
}

impl<'a> Events<'a> {
    pub fn new(
        consent: DataCollectionConsent,
        ids: DeviceOSIds,
        data: &'a [TelemetryEvent],
    ) -> Self {
        Self {
            data,
            data_header: data_header(consent, ids),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

pub fn remove_event(event: &mut TelemetryEvent) {
    if let Some(state) = event.state_mut() {
        *state = State::Removed;
    }
    event.clear_options();

    if let TelemetryEvent::HwPeripheralUsb(event) = event {
        event.timestamp = date_time();
    }
    // TODO: any other types with timestamp, etc.
}

pub fn diff(events: &mut Vec<TelemetryEvent>, old_events: &[TelemetryEvent]) {
    // TODO: warn if multiple things have same primary key?

    let mut m1 = HashMap::new();
    for (n, event) in events.iter_mut().enumerate() {
        m1.insert((event.type_(), event.primaries()), (n, event));
    }

    let mut m2 = HashMap::new();
    for event in old_events.iter() {
        m2.insert((event.type_(), event.primaries()), event);
    }

    let mut added_updated = HashSet::new();
    for (k, (n, new)) in m1.iter_mut() {
        if let Some(old) = m2.get(k) {
            if new.diff(old) {
                if let Some(state) = new.state_mut() {
                    *state = State::Updated;
                }
                added_updated.insert(*n);
            }
        } else {
            if let Some(state) = new.state_mut() {
                *state = State::Added;
            }
            added_updated.insert(*n);
        }
    }

    let mut new_events = Vec::new();
    for (k, old) in m2.iter_mut() {
        if !m1.contains_key(k) {
            let mut new = (**old).clone();
            remove_event(&mut new);
            new_events.push(new);
        }
    }

    let mut i = 0;
    events.retain(|_| {
        i += 1;
        added_updated.contains(&(i - 1))
    });
    events.extend(new_events);
}
