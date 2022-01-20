use nix::sys::utsname::uname;
use os_release::OsRelease;
use std::str::FromStr;

pub mod api;
pub mod event;
pub mod report;

use event::{read_file, unknown, AnyTelemetryEventEnum, TelemetryEventType};
use report::ReportFreq;

pub struct EventDesc {
    freq: ReportFreq,
    cb: fn(&mut Vec<AnyTelemetryEventEnum>),
}

impl EventDesc {
    fn new(freq: ReportFreq, cb: fn(&mut Vec<AnyTelemetryEventEnum>)) -> Self {
        Self { freq, cb }
    }

    pub fn freq(&self) -> ReportFreq {
        self.freq
    }

    pub fn generate(&self, events: &mut Vec<AnyTelemetryEventEnum>) {
        (self.cb)(events)
    }
}

pub fn event(type_: TelemetryEventType) -> Option<EventDesc> {
    Some(match type_ {
        TelemetryEventType::SwLinuxKernel => EventDesc::new(ReportFreq::Daily, |events| {
            let utsname = uname();
            events.push(
                event::LinuxKernel {
                    name: utsname.sysname().to_string(),
                    release: utsname.release().to_string(),
                    state: event::Swstate::Same, // TODO
                    version: utsname.version().to_string(),
                }
                .into(),
            );
        }),
        TelemetryEventType::HwBattery => EventDesc::new(ReportFreq::Daily, |events| {
            events.push(
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
                .into(),
            );
        }),
        TelemetryEventType::HwBaseBoard => EventDesc::new(ReportFreq::Daily, |events| {
            events.push(
                event::BaseBoard {
                    base_board_id: read_file("/sys/class/dmi/id/board_name"),
                    ct_number: String::new(), // XXX
                    manufacturer: read_file("/sys/class/dmi/id/board_vendor"),
                    state: event::Hwstate::Same, // TODO
                    version: read_file("/sys/class/dmi/id/board_version"),
                }
                .into(),
            );
        }),
        TelemetryEventType::SwFirmware => EventDesc::new(ReportFreq::Daily, |events| {
            events.push(
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
                .into(),
            );
        }),
        TelemetryEventType::HwSystem => EventDesc::new(ReportFreq::Daily, |events| {
            events.push(
                event::System {
                    capabilities: None, // XXX
                    chassis: read_file("/sys/class/dmi/id/chassis_type"),
                    family: read_file("/sys/class/dmi/id/product_family"),
                    feature_byte: None, // XXX
                    manufacturer: read_file("/sys/class/dmi/id/sys_vendor"),
                    model: read_file("/sys/class/dmi/id/product_name"),
                    serialnumber: read_file("/sys/class/dmi/id/product_serial")
                        .unwrap_or_else(unknown),
                    sku: read_file("/sys/class/dmi/id/product_sku"),
                    state: event::Hwstate::Same, // XXX,
                    uuid: read_file("/sys/class/dmi/id/product_uuid").unwrap_or_else(unknown),
                    version: read_file("/sys/class/dmi/id/product_version"),
                    width: None, // XXX
                }
                .into(),
            );
        }),
        TelemetryEventType::SwOperatingSystem => EventDesc::new(ReportFreq::Daily, |events| {
            let os_release = OsRelease::new().ok();
            events.push(
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
                .into(),
            );
        }),
        TelemetryEventType::SwDriver => EventDesc::new(ReportFreq::Daily, |events| {
            if let Some(modules) = read_file::<_, String>("/proc/modules") {
                for line in modules.lines() {
                    let mut cols = line.split(' ');
                    let module_name = cols.next().unwrap_or("unknown");
                    let size = cols.next().and_then(|s| i64::from_str(s).ok());
                    let _instances = cols.next();
                    let _deps = cols.next();
                    let _state = cols.next();
                    events.push(
                        event::Driver {
                            display_name: None,         // XXX
                            driver_category: None,      // XXX
                            driver_type: String::new(), // XXX
                            driver_version: None,       // XXX
                            link_time: None,            // XXX
                            module_name: module_name.to_string(),
                            pnp_device_id: None, // XXX
                            size,
                            state: event::Swstate::Same, // XXX
                        }
                        .into(),
                    );
                }
            }
        }),
        _ => return None,
    })
}
