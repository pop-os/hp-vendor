use nix::sys::utsname::uname;
use os_release::OsRelease;
use time::{format_description, OffsetDateTime};

pub mod event;

use event::{AnyTelemetryEventEnum, TelemetryEventType};

fn unknown() -> String {
    "unknown".to_string()
}

pub fn data_header() -> event::TelemetryHeaderModel {
    let (os_name, os_version) = match OsRelease::new() {
        Ok(OsRelease { name, version, .. }) => (name, version),
        Err(_) => (unknown(), unknown()),
    };

    // XXX offset format? Fraction?
    let format =
        format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]").unwrap();
    let (timestamp, timestamp_utc_offset) = match OffsetDateTime::now_local() {
        Ok(time) => (
            time.format(&format).ok().unwrap_or_else(unknown),
            time.offset().whole_hours().into(),
        ),
        Err(_) => (unknown(), 0),
    };

    event::TelemetryHeaderModel {
        consent: event::DataCollectionConsent {
            level: String::new(), // TODO
        },
        data_provider: event::DataProviderInfo {
            app_name: env!("CARGO_PKG_NAME").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_name,
            os_version,
        },
        ids: event::DeviceOSIds {
            bios_uuid: String::new(),     // TODO
            device_id: String::new(),     // TODO
            os_install_id: String::new(), // TODO
        },
        timestamp,
        timestamp_utc_offset,
    }
}

pub fn event(type_: TelemetryEventType) -> Option<AnyTelemetryEventEnum> {
    Some(match type_ {
        TelemetryEventType::SwLinuxKernel => {
            let utsname = uname();
            event::LinuxKernel {
                name: utsname.sysname().to_string(),
                release: utsname.release().to_string(),
                state: event::Swstate::Same, // TODO
                version: utsname.version().to_string(),
            }
        }
        .into(),
        _ => return None,
    })
}
