use nix::sys::utsname::uname;
use os_release::OsRelease;

use std::{collections::HashSet, fmt::Write, fs, path::Path, str::FromStr};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub mod api;
pub mod event;
pub mod report;

use event::{read_file, unknown, TelemetryEvent, TelemetryEventType};
use report::ReportFreq;

const PCI_EXT_CAP_ID_DSN: u16 = 0x03;

fn dmi() -> Vec<dmi::Table> {
    if let Ok(data) = fs::read("/sys/firmware/dmi/tables/DMI") {
        dmi::tables(&data)
    } else {
        Vec::new()
    }
}

pub fn pcie_dsn<P: AsRef<Path>>(path: P) -> Option<String> {
    let data = std::fs::read(path.as_ref()).ok()?;
    let mut been = HashSet::new();
    let mut offset = 0x100;
    while offset != 0 && been.insert(offset) {
        macro_rules! entry {
            ($offset:expr) => {
                *data.get(offset + $offset)?
            };
        }
        let next = ((entry![3] as u16) << 4) | ((entry![2] as u16) >> 4);
        let _version = entry![2] & 0xf;
        let cap_id = entry![0] as u16 | ((entry![1] as u16) << 8);
        if cap_id == PCI_EXT_CAP_ID_DSN {
            let mut dsn = String::new();
            for i in data.get(offset as usize + 4..offset as usize + 12)? {
                write!(dsn, "{:02x}-", i).ok()?;
            }
            dsn.pop();
            return Some(dsn);
        }
        offset = next.into();
    }
    None
}

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
                    state: event::Swstate::Installed,
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
                    state: event::Hwstate::Added,
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
                    state: event::Hwstate::Added,
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
                    rom_size: None,       // XXX
                    runtime_size: None,   // XXX
                    smbios_version: None, // XXX
                    state: event::Swstate::Installed,
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
                    state: event::Hwstate::Added,
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
                    sku: None, // XXX
                    state: event::Swstate::Installed,
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
                            state: event::Swstate::Installed,
                        }
                        .into(),
                    );
                }
            }
        }),
        TelemetryEventType::HwNvmeStoragePhysical => EventDesc::new(ReportFreq::Daily, |events| {
            let entries = fs::read_dir("/sys/class/nvme");
            for i in entries.into_iter().flatten().filter_map(Result::ok) {
                let path = i.path();
                events.push(
                    event::NvmestoragePhysical {
                        bus_info: read_file(path.join("address")),
                        firmware_revision: read_file(path.join("firmware_rev")),
                        model: read_file(path.join("model")),
                        serial_number: read_file(path.join("serial")).unwrap_or_else(unknown),
                        state: event::Hwstate::Added,
                        sub_system_id: read_file(path.join("device/subsystem_vendor")),
                        total_capacity: None, // XXX
                        vendor_id: read_file(path.join("device/vendor")),
                    }
                    .into(),
                );
            }
        }),
        TelemetryEventType::HwNvmeStorageLogical => EventDesc::new(ReportFreq::Daily, |events| {
            let entries = fs::read_dir("/sys/class/block");
            for i in entries.into_iter().flatten().filter_map(Result::ok) {
                if let Some(name) = i.file_name().to_str() {
                    if name.starts_with("nvme") && !name.contains('p') {
                        let path = i.path();

                        let entries = fs::read_dir("/sys/class/block");
                        let partitions = entries
                            .into_iter()
                            .flatten()
                            .filter_map(Result::ok)
                            .filter_map(|i| {
                                let file_name = i.file_name();
                                let path = i.path();
                                let number = match (|| {
                                    let part_name = file_name.to_str()?;
                                    let number = part_name.strip_prefix(name)?.strip_prefix('p')?;
                                    let number: i64 = number.parse().ok()?;
                                    Some(number)
                                })() {
                                    Some(number) => number,
                                    None => {
                                        return None;
                                    }
                                };
                                Some(event::StoragePartition {
                                    file_system: String::new(), // XXX
                                    flags: Vec::new(),          // XXX
                                    name: String::new(),        // XXX
                                    number,
                                    size: read_file(path.join("size")).unwrap_or(0),
                                })
                            })
                            .collect();

                        events.push(
                            event::NvmestorageLogical {
                                lba_size: None,         // XXX
                                node_id: String::new(), // XXX
                                partitions: Some(partitions),
                                serial_number: read_file(path.join("device/serial"))
                                    .unwrap_or_else(unknown),
                                used_capacity: None, // XXX
                            }
                            .into(),
                        );
                    }
                }
            }
        }),
        TelemetryEventType::HwMemoryPhysical => EventDesc::new(ReportFreq::Daily, |events| {
            for i in dmi() {
                if let Some(info) = i.get::<dmi::MemoryDevice>() {
                    let form_factor = match info.form_factor {
                        0x01 => "Other",
                        0x02 => "Unknown",
                        0x03 => "SIMM",
                        0x04 => "SIP",
                        0x05 => "Chip",
                        0x06 => "DIP",
                        0x07 => "ZIP",
                        0x08 => "Proprietary Card",
                        0x09 => "DIMM",
                        0x0A => "TSOP",
                        0x0B => "Row of chips",
                        0x0C => "RIMM",
                        0x0D => "SODIMM",
                        0x0E => "SRIMM",
                        0x0F => "FB-DIMM",
                        0x10 => "Die",
                        _ => "Unknown",
                    }
                    .to_string();
                    let type_ = match info.memory_kind {
                        0x01 => "Other",
                        0x02 => "Unknown",
                        0x03 => "DRAM",
                        0x04 => "EDRAM",
                        0x05 => "VRAM",
                        0x06 => "SRAM",
                        0x07 => "RAM",
                        0x08 => "ROM",
                        0x09 => "FLASH",
                        0x0A => "EEPROM",
                        0x0B => "FEPROM",
                        0x0C => "EPROM",
                        0x0D => "CDRAM",
                        0x0E => "3DRAM",
                        0x0F => "SDRAM",
                        0x10 => "SGRAM",
                        0x11 => "RDRAM",
                        0x12 => "DDR",
                        0x13 => "DDR2",
                        0x14 => "DDR2 FB-DIMM",
                        0x18 => "DDR3",
                        0x19 => "DBD2",
                        0x1A => "DDR4",
                        0x1B => "LPDDR",
                        0x1C => "LPDDR2",
                        0x1D => "LPDDR3",
                        _ => "Unknown",
                    }
                    .to_string();
                    events.push(
                        event::MemoryPhysical {
                            bank_label: i.get_str(info.bank_locator).cloned(),
                            data_width: Some(info.data_width.into()),
                            form_factor: Some(form_factor),
                            locator: i.get_str(info.device_locator).cloned(),
                            manufacturer: i.get_str(info.manufacturer).cloned(),
                            part_number: i
                                .get_str(info.part_number)
                                .cloned()
                                .unwrap_or_else(unknown),
                            rank: Some((info.attributes & 0b1111).into()),
                            serial_number: i
                                .get_str(info.serial_number)
                                .cloned()
                                .unwrap_or_else(unknown),
                            size: Some(info.size.into()),
                            speed: Some(info.speed.into()),
                            state: event::Hwstate::Added,
                            type_: Some(type_),
                        }
                        .into(),
                    )
                }
            }
        }),
        _ => return None,
    })
}

pub fn all_events() -> Vec<event::TelemetryEvent> {
    let mut events = Vec::new();
    for i in event::TelemetryEventType::iter() {
        if let Some(event) = event(i) {
            event.generate(&mut events);
        }
    }
    events
}
