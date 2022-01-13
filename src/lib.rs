use nix::sys::utsname::uname;
use os_release::OsRelease;
use std::{fs, path::Path, str::FromStr};
use time::OffsetDateTime;

pub mod event;
pub mod report;

use event::{AnyTelemetryEventEnum, TelemetryEventType};
use report::ReportFreq;

pub fn read_file<P: AsRef<Path>, T: FromStr>(path: P) -> Option<T> {
    fs::read_to_string(path).ok().and_then(|x| x.parse().ok())
}

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
        TelemetryEventType::HwBattery => EventDesc::new(ReportFreq::Daily, || {
            event::Battery {
                ct_number: String::new(), // XXX,
                devicename: read_file("/sys/class/power_supply/BAT0/model_name"),
                energy_design: read_file("/sys/class/power_supply/BAT0/charge_full_design")
                    .map(|x: f64| x / 1000000.), // XXX divisor?
                manufacturer: read_file("/sys/class/power_supply/BAT0/manufacturer"),
                serial_number: read_file("/sys/class/power_supply/BAT0/serial_number")
                    .unwrap_or_else(unknown),
                state: event::Hwstate::Same, // TODO
                voltage_design: read_file("/sys/class/power_supply/BAT0/voltage_min_design")
                    .map(|x: f64| x / 1000.),
            }
            .into()
        }),
        TelemetryEventType::HwBaseBoard => EventDesc::new(ReportFreq::Daily, || {
            event::BaseBoard {
                base_board_id: read_file("/sys/class/dmi/id/board_name"),
                ct_number: String::new(), // XXX
                manufacturer: read_file("/sys/class/dmi/id/board_vendor"),
                state: event::Hwstate::Same, // TODO
                version: read_file("/sys/class/dmi/id/board_version"),
            }
            .into()
        }),
        TelemetryEventType::SwFirmware => EventDesc::new(ReportFreq::Daily, || {
            event::Firmware {
                address: None, // XXX
                bios_release_date: read_file("/sys/class/dmi/id/bios_date"),
                bios_vendor: read_file("/sys/class/dmi/id/bios_vendor"),
                bios_version: read_file("/sys/class/dmi/id/bios_version"),
                capabilities: None, // XXX
                embedded_controller_version: read_file("/sys/class/dmi/id/ec_firmware_release"),
                rom_size: None,              // XXX
                runtime_size: None,          // XXX
                smbios_version: None,        // XXX
                state: event::Swstate::Same, // XXX
            }
            .into()
        }),
        TelemetryEventType::HwSystem => EventDesc::new(ReportFreq::Daily, || {
            event::System {
                capabilities: None, // XXX
                chassis: read_file("/sys/class/dmi/id/chassis_type"),
                family: read_file("/sys/class/dmi/id/product_family"),
                feature_byte: None, // XXX
                manufacturer: read_file("/sys/class/dmi/id/sys_vendor"),
                model: read_file("/sys/class/dmi/id/product_name"),
                serialnumber: read_file("/sys/class/dmi/id/product_serial").unwrap_or_else(unknown),
                sku: read_file("/sys/class/dmi/id/product_sku"),
                state: event::Hwstate::Same, // XXX,
                uuid: read_file("/sys/class/dmi/id/product_uuid").unwrap_or_else(unknown),
                version: read_file("/sys/class/dmi/id/product_version"),
                width: None, // XXX
            }
            .into()
        }),
        TelemetryEventType::SwOperatingSystem => EventDesc::new(ReportFreq::Daily, || {
            let os_release = OsRelease::new().ok();
            event::OperatingSystem {
                boot_device: String::new(), // XXX
                codename: os_release.as_ref().map(|x| x.version_codename.to_owned()),
                manufacturer: None, // XXX
                name: os_release
                    .as_ref()
                    .map_or_else(unknown, |x| x.name.to_owned()),
                sku: None,                   // XXX
                state: event::Swstate::Same, // XXX
                version: os_release.map(|x| x.version.clone()),
            }
            .into()
        }),
        _ => return None,
    })
}
