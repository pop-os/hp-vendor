use nix::sys::utsname::uname;
use os_release::OsRelease;
use std::{fs, str::FromStr};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub mod api;
pub mod event;
pub mod report;

use event::{read_file, unknown, TelemetryEvent, TelemetryEventType};
use report::ReportFreq;

pub struct EventDesc {
    freq: ReportFreq,
    cb: fn(&mut Vec<TelemetryEvent>),
}

impl EventDesc {
    fn new(freq: ReportFreq, cb: fn(&mut Vec<TelemetryEvent>)) -> Self {
        Self { freq, cb }
    }

    pub fn freq(&self) -> ReportFreq {
        self.freq
    }

    pub fn generate(&self, events: &mut Vec<TelemetryEvent>) {
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
        TelemetryEventType::HwBatteryUsage => EventDesc::new(ReportFreq::Daily, |events| {
            // XXX: Division? Integers?
            fn energy_rate() -> Option<i64> {
                let current: i64 = read_file("/sys/class/power_supply/BAT0/current_now")?;
                let voltage: i64 = read_file("/sys/class/power_supply/BAT0/voltage_now")?;
                Some(current * voltage / 1000000)
            }
            let timestamp = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .ok()
                .unwrap_or_else(unknown);
            events.push(
                event::BatteryUsage {
                    battery_state: read_file("/sys/class/power_supply/BAT0/status")
                        .unwrap_or_else(unknown),
                    cell_voltage: None,       // XXX
                    ct_number: String::new(), // XXX
                    cycle_count: read_file("/sys/class/power_supply/BAT0/cycle_count")
                        .unwrap_or(-1),
                    eletric_current: None, // XXX
                    energy_full: read_file("/sys/class/power_supply/BAT0/charge_full")
                        .map(|x: f64| x / 1000000.)
                        .unwrap_or(-1.),
                    energy_rate: energy_rate(),
                    energy_remaining: read_file("/sys/class/power_supply/BAT0/charge_now")
                        .map(|x: f64| x / 1000000.)
                        .unwrap_or(-1.),
                    max_error: None, // XXX
                    serial_number: read_file("/sys/class/power_supply/BAT0/serial_number")
                        .unwrap_or_else(unknown),
                    status_register: None, // XXX
                    temperature: None,     // XXX
                    time_to_empty: None,   // XXX
                    timestamp,
                    voltage: read_file("/sys/class/power_supply/BAT0/voltage_now")
                        .map(|x: i64| x / 1000000),
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
            fn bios_date() -> Option<String> {
                let date: String = read_file("/sys/class/dmi/id/bios_date")?;
                let mut parts = date.split('/');
                let month = parts.next()?;
                let day = parts.next()?;
                let year = parts.next()?;
                Some(format!("{}-{}-{}", year, month, day))
            }
            events.push(
                event::Firmware {
                    address: None, // XXX
                    bios_release_date: bios_date(),
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
        TelemetryEventType::HwNvmeStoragePhysical => EventDesc::new(ReportFreq::Daily, |events| {
            let entries = fs::read_dir("/sys/class/block");
            for i in entries.into_iter().flatten().filter_map(Result::ok) {
                if let Some(name) = i.file_name().to_str() {
                    if name.starts_with("nvme") {
                        let path = i.path();
                        events.push(
                            event::NvmestoragePhysical {
                                bus_info: read_file(path.join("device/address")),
                                firmware_revision: read_file(path.join("device/firmware_rev")),
                                model: read_file(path.join("device/model")),
                                serial_number: read_file(path.join("device/serial"))
                                    .unwrap_or_else(unknown),
                                state: event::Hwstate::Same, // XXX
                                sub_system_id: None,         // XXX
                                total_capacity: None,        // XXX
                                vendor_id: None,             // XXX
                            }
                            .into(),
                        );
                    }
                }
            }
        }),
        _ => return None,
    })
}
