use nix::sys::utsname::uname;
use os_release::OsRelease;
use time::{format_description, OffsetDateTime};

pub mod event;
pub mod report;

use event::{AnyTelemetryEventEnum, TelemetryEventType};
use report::ReportFreq;

pub struct EventDesc {
    freq: ReportFreq,
    cb: fn() -> AnyTelemetryEventEnum,
}

impl EventDesc {
    fn new(freq: ReportFreq, cb: fn() -> AnyTelemetryEventEnum) -> Self {
        Self { freq, cb }
    }

    pub fn freq(&self) -> ReportFreq {
        self.freq
    }

    pub fn generate(&self) -> AnyTelemetryEventEnum {
        (self.cb)()
    }
}

fn unknown() -> String {
    "unknown".to_string()
}

pub fn data_header() -> event::TelemetryHeaderModel {
    let (os_name, os_version) = match OsRelease::new() {
        Ok(OsRelease { name, version, .. }) => (name, version),
        Err(_) => (unknown(), unknown()),
    };

    // XXX offset format? Fraction?
    let format = time::format_description::well_known::Rfc3339;
    let timestamp = OffsetDateTime::now_utc()
        .format(&format)
        .ok()
        .unwrap_or_else(unknown);

    event::TelemetryHeaderModel {
        consent: event::DataCollectionConsent {
            opted_in_level: String::new(), // XXX
            version: String::new(),        // XXX
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
    }
}

pub fn event(type_: TelemetryEventType) -> Option<EventDesc> {
    Some(match type_ {
        TelemetryEventType::SwLinuxKernel => EventDesc::new(ReportFreq::Daily, || {
            let utsname = uname();
            event::LinuxKernel {
                name: utsname.sysname().to_string(),
                release: utsname.release().to_string(),
                state: event::Swstate::Same, // TODO
                version: utsname.version().to_string(),
            }
            .into()
        }),
        _ => return None,
    })
}
