use nix::sys::utsname::uname;
use os_release::OS_RELEASE;
use plain::Plain;
use std::{
    collections::HashMap, convert::TryInto, ffi::OsStr, fs, io, path::PathBuf, process::Command,
    str::FromStr,
};

pub mod api;
pub mod cmd;
pub mod config;
mod db;
pub mod event;
mod frequency;
mod util;

use config::SamplingFrequency;
use event::{read_file, unknown, State, TelemetryEvent, TelemetryEventType};
use frequency::Frequencies;
use util::{
    dmi::{dmi, CacheInfo21},
    drm::DrmDevice,
};

pub fn supported_hardware() -> Result<(), String> {
    let board_vendor: String =
        read_file("/sys/class/dmi/id/board_vendor").ok_or_else(|| "`board_vendor` not defined")?;
    let board_name: String =
        read_file("/sys/class/dmi/id/board_name").ok_or_else(|| "`board_name` not defined")?;
    if (board_vendor.as_str(), board_name.as_str()) != ("HP", "8A78") {
        Err(format!("`{} {}` unrecognized", board_vendor, board_name))
    } else {
        Ok(())
    }
}

fn battery() -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/class/power_supply").ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if let Ok(type_) = fs::read(path.join("type")) {
            let scope = fs::read(path.join("scope")).ok();
            if type_ == b"Battery\n" && scope.as_deref() != Some(b"Device\n") {
                return Some(path);
            }
        }
    }
    None
}

pub struct PeriodicEventDesc {
    cb: fn(&mut Vec<TelemetryEvent>),
}

impl PeriodicEventDesc {
    pub fn generate(&self, events: &mut Vec<TelemetryEvent>) {
        (self.cb)(events);
    }
}

pub struct UdevEventDesc {
    subsystem: &'static str,
    cb: fn(&mut Vec<TelemetryEvent>, &udev::Device),
}

impl UdevEventDesc {
    pub fn generate(&self, events: &mut Vec<TelemetryEvent>, device: &udev::Device) {
        (self.cb)(events, device);
    }
}

pub enum EventDesc {
    Periodic(PeriodicEventDesc),
    Udev(UdevEventDesc),
}

impl EventDesc {
    fn new(cb: fn(&mut Vec<TelemetryEvent>)) -> Self {
        Self::Periodic(PeriodicEventDesc { cb })
    }

    fn new_udev(subsystem: &'static str, cb: fn(&mut Vec<TelemetryEvent>, &udev::Device)) -> Self {
        Self::Udev(UdevEventDesc { subsystem, cb })
    }
}

struct UdevDescs(HashMap<&'static str, Vec<UdevEventDesc>>);

impl UdevDescs {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn insert(&mut self, desc: UdevEventDesc) {
        self.0
            .entry(desc.subsystem)
            .or_insert_with(Vec::new)
            .push(desc);
    }

    fn get(&self, subsystem: &str) -> &[UdevEventDesc] {
        self.0.get(subsystem).map_or(&[], Vec::as_slice)
    }
}

pub fn event(type_: TelemetryEventType) -> Option<EventDesc> {
    Some(match type_ {
        TelemetryEventType::SwLinuxKernel => EventDesc::new(|events| {
            let utsname = uname();
            events.push(
                event::LinuxKernel {
                    name: utsname.sysname().to_string(),
                    release: utsname.release().to_string(),
                    version: utsname.version().to_string(),
                }
                .into(),
            );
        }),
        TelemetryEventType::HwBattery => EventDesc::new(|events| {
            let path = match battery() {
                Some(path) => path,
                None => return,
            };

            events.push(
                event::Battery {
                    ct_number: String::new(), // XXX,
                    devicename: read_file(path.join("model_name")),
                    energy_design: read_file(path.join("charge_full_design"))
                        .map(|x: i64| x / 1000),
                    manufacturer: read_file(path.join("manufacturer")),
                    serial_number: read_file(path.join("serial_number")).unwrap_or_else(unknown),
                    state: State::Added,
                    voltage_design: read_file(path.join("voltage_min_design"))
                        .map(|x: i64| x / 1000),
                }
                .into(),
            );
        }),
        // TODO: generate in daemon
        TelemetryEventType::HwBatteryLife => EventDesc::new(|events| {
            let path = match battery() {
                Some(path) => path,
                None => return,
            };

            events.push(
                event::BatteryLife {
                    ct_number: String::new(), // XXX
                    cycle_count: read_file(path.join("cycle_count")).unwrap_or(-1),
                    energy_full: read_file(path.join("charge_full"))
                        .map(|x: i64| x / 1000)
                        .unwrap_or(-1),
                    serial_number: read_file(path.join("serial_number")).unwrap_or_else(unknown),
                    total_ac_charging_time: None, // XXX
                    total_ac_time: 0,             // XXX
                    total_dc_time: 0,             // XXX
                }
                .into(),
            );
        }),
        TelemetryEventType::HwBaseBoard => EventDesc::new(|events| {
            events.push(
                event::BaseBoard {
                    base_board_id: read_file("/sys/class/dmi/id/board_name"),
                    ct_number: String::new(), // XXX
                    manufacturer: read_file("/sys/class/dmi/id/board_vendor"),
                    state: State::Added,
                    version: read_file("/sys/class/dmi/id/board_version"),
                }
                .into(),
            );
        }),
        TelemetryEventType::SwFirmware => EventDesc::new(|events| {
            for i in dmi() {
                if let Some(bios) = i.get::<util::dmi::BiosInfo31>() {
                    let bios_date = (|| {
                        let date = i.get_str(bios.date)?;
                        let mut parts = date.split('/');
                        let month = parts.next()?;
                        let day = parts.next()?;
                        let year = parts.next()?;
                        Some(format!("{}-{}-{}", year, month, day))
                    })();
                    let ec_version = format!("{}.{}", bios.ec_major, bios.ec_minor);
                    // XXX not working?
                    let smbios_version = (|| {
                        let entry_point =
                            fs::read("/sys/firmware/dmi/tables/smbios_entry_point").ok()?;
                        let smbios = dmi::Smbios::from_bytes(&entry_point).ok()?;
                        Some(format!("{}.{}", smbios.major_version, smbios.minor_version))
                    })();
                    let mut rom_size = (bios.rom_size as u16 + 1) / 16;
                    if bios.rom_size == 0xff {
                        let unit = bios.extended_rom_size >> 14;
                        let size = bios.extended_rom_size & 0x3fff;
                        if unit == 0b00 {
                            rom_size = size;
                        } else if unit == 0b01 {
                            rom_size = size * 1024;
                        }
                    }
                    events.push(
                        event::Firmware {
                            bios_release_date: bios_date,
                            bios_vendor: i.get_str(bios.vendor).cloned(),
                            bios_version: i.get_str(bios.version).cloned(),
                            capabilities: None, // XXX
                            embedded_controller_version: Some(ec_version),
                            rom_size: Some(rom_size.to_string()), // XXX why string?
                            smbios_version,
                        }
                        .into(),
                    );

                    break;
                }
            }
        }),
        TelemetryEventType::HwSystem => EventDesc::new(|events| {
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
                    state: State::Added,
                    uuid: read_file("/sys/class/dmi/id/product_uuid").unwrap_or_else(unknown),
                    version: read_file("/sys/class/dmi/id/product_version"),
                }
                .into(),
            );
        }),
        TelemetryEventType::SwOperatingSystem => EventDesc::new(|events| {
            let os_release = OS_RELEASE.as_ref().ok();
            events.push(
                event::OperatingSystem {
                    boot_device: String::new(), // XXX
                    codename: os_release.as_ref().map(|x| x.version_codename.to_owned()),
                    name: os_release
                        .as_ref()
                        .map_or_else(unknown, |x| x.name.to_owned()),
                    version: os_release.map(|x| x.version.clone()),
                }
                .into(),
            );
        }),
        TelemetryEventType::SwDriver => EventDesc::new(|events| {
            if let Some(modules) = read_file::<_, String>("/proc/modules") {
                for line in modules.lines() {
                    let mut cols = line.split(' ');
                    let module_name = cols.next().unwrap_or("unknown");
                    let size = cols.next().and_then(|s| i64::from_str(s).ok());
                    let _instances = cols.next();
                    let _deps = cols.next();
                    let _state = cols.next();
                    let modinfo = |field| {
                        let res = Command::new("/usr/sbin/modinfo")
                            .args(["-F", field, module_name])
                            .output()
                            .ok()?;
                        let mut s = String::from_utf8(res.stdout).ok()?;
                        s.truncate(s.trim_end().len());
                        if !res.status.success() || s.is_empty() {
                            return None;
                        }
                        Some(s)
                    };
                    events.push(
                        event::Driver {
                            author: modinfo("author"),
                            description: modinfo("description"),
                            driver_version: modinfo("version"),
                            link_time: None, // XXX
                            module_name: module_name.to_string(),
                            module_path: modinfo("filename").unwrap_or_else(unknown),
                            module_type: String::new(), // XXX
                            size,
                        }
                        .into(),
                    );
                }
            }
        }),
        TelemetryEventType::HwNvmeStoragePhysical => {
            EventDesc::new_udev("nvme", |events, device| {
                let path = device.syspath();
                events.push(
                    event::NvmestoragePhysical {
                        bus_info: read_file(path.join("address")),
                        firmware_revision: read_file(path.join("firmware_rev")),
                        model: read_file(path.join("model")),
                        serial_number: read_file(path.join("serial")).unwrap_or_else(unknown),
                        state: State::Added,
                        sub_system_id: read_file(path.join("device/subsystem_vendor")),
                        total_capacity: None, // XXX
                        vendor_id: read_file(path.join("device/vendor")),
                    }
                    .into(),
                );
            })
        }
        TelemetryEventType::HwNvmeStorageLogical => {
            EventDesc::new_udev("block", |events, device| {
                fn partitions(device: &udev::Device) -> io::Result<Vec<event::StoragePartition>> {
                    let mut enumerator = udev::Enumerator::new()?;
                    enumerator.match_parent(device)?;
                    Ok(enumerator
                        .scan_devices()
                        .into_iter()
                        .flatten()
                        .filter_map(|child| {
                            let path = child.syspath();
                            if child.devtype().and_then(OsStr::to_str) != Some("partition") {
                                return None;
                            }
                            let number = match child.sysnum() {
                                Some(number) => number as i64,
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
                        .collect())
                }

                if let Some(name) = device.sysname().to_str() {
                    if name.starts_with("nvme")
                        && device.devtype().and_then(OsStr::to_str) == Some("disk")
                    {
                        let path = device.syspath();

                        events.push(
                            event::NvmestorageLogical {
                                lba_size: None,         // XXX
                                node_id: String::new(), // XXX
                                partitions: partitions(device).ok(),
                                serial_number: read_file(path.join("device/serial"))
                                    .unwrap_or_else(unknown),
                                used_capacity: None, // XXX
                            }
                            .into(),
                        );
                    }
                }
            })
        }
        // XXX need support for multiple events with same selector
        TelemetryEventType::HwNvmeSmartLog => EventDesc::new_udev("nvme", |events, device| {
            let path = device.syspath();
            let devnode = match device.devnode() {
                Some(devnode) => devnode,
                None => {
                    return;
                }
            };

            if let Some(smart_log) = util::nvme::smart_log(devnode) {
                events.push(
                    event::NvmesmartLog {
                        available_spare: smart_log.avail_spare,
                        available_spare_threshold: smart_log.spare_thresh,
                        controller_busy_time: smart_log
                            .controller_busy_time
                            .try_into()
                            .unwrap_or(-1),
                        critical_composite_temperature_threshold: -1, // XXX
                        critical_composite_temperature_time: smart_log.critical_comp_time,
                        critical_warning: smart_log.critical_warning,
                        data_units_read: smart_log.data_units_read.try_into().unwrap_or(-1),
                        data_units_written: smart_log.data_units_written.try_into().unwrap_or(-1),
                        endurance_critical_warning: -1, // XXX
                        host_read_commands: smart_log.host_read_commands.try_into().unwrap_or(-1),
                        host_write_commands: smart_log.host_write_commands.try_into().unwrap_or(-1),
                        media_errors: smart_log.media_errors.try_into().unwrap_or(-1),
                        num_err_log_entries: smart_log.num_err_log_entries.try_into().unwrap_or(-1),
                        nvme_version: String::new(), // XXX
                        percentage_used: smart_log.percent_used,
                        power_cycles: smart_log.power_cycles.try_into().unwrap_or(-1),
                        power_on_hours: smart_log.power_on_hours.try_into().unwrap_or(-1),
                        serial_number: read_file(path.join("serial")).unwrap_or_else(unknown),
                        temperature_sensor: Vec::new(), // XXX
                        thermal_management_total_time: Vec::new(), // XXX
                        thermal_management_trans_count: Vec::new(), // XXX
                        unsafe_shutdowns: smart_log.unsafe_shutdowns.try_into().unwrap_or(-1),
                        warning_temperature_threshold: -1, // XXX
                        warning_temperature_time: smart_log.warning_temp_time,
                    }
                    .into(),
                );
            }
        }),
        TelemetryEventType::HwPeripheralUsb => EventDesc::new_udev("usb", |events, device| {
            let path = device.syspath();

            if device.devtype().and_then(OsStr::to_str) != Some("usb_device")
                || !path.join("idProduct").exists()
            {
                return;
            }

            events.push(
                event::PeripheralUSB {
                    device_version: None, // XXX
                    manufacturer: read_file(path.join("manufacturer")),
                    manufacturer_id: read_file(path.join("idVendor")),
                    message: None, // XXX
                    product: read_file(path.join("product")),
                    product_id: read_file(path.join("idProduct")),
                    state: State::Added,
                    timestamp: event::date_time(),
                    usb_bus_id: read_file(path.join("busnum")).unwrap_or(0),
                    usb_device_id: read_file(path.join("devnum")).unwrap_or_else(unknown),
                    usb_speed: read_file(path.join("speed")).unwrap_or_else(unknown),
                }
                .into(),
            )
        }),
        TelemetryEventType::HwMemoryPhysical => EventDesc::new(|events| {
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
                            state: State::Added,
                            type_: Some(type_),
                        }
                        .into(),
                    )
                }
            }
        }),
        TelemetryEventType::HwProcessor => EventDesc::new(|events| {
            let dmi = dmi();
            for i in &dmi {
                if let Some(processor) = i.get::<dmi::ProcessorInfo>() {
                    let mut cache_infos = Vec::new();
                    for i in [
                        processor.l1_cache_handle,
                        processor.l2_cache_handle,
                        processor.l3_cache_handle,
                    ] {
                        if i == 0 {
                            continue;
                        }
                        if let Some(cache) = dmi.iter().find(|x| x.header.handle == i) {
                            if let Some(cache_info) = cache.get::<CacheInfo21>() {
                                cache_infos.push(cache_info);
                                // Seems to handle non-unified L1
                                if cache_info.socket != 0 {
                                    for j in &dmi {
                                        if j.header.handle == i {
                                            continue;
                                        }
                                        if let Some(other_cache_info) = j.get::<CacheInfo21>() {
                                            if cache.get_str(other_cache_info.socket)
                                                == j.get_str(cache_info.socket)
                                            {
                                                cache_infos.push(other_cache_info)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let caches = cache_infos
                        .iter()
                        .map(|cache_info| {
                            let level = (cache_info.configuration & 0b111) + 1;
                            let type_ = match cache_info.system_cache_type {
                                0x01 => "Other",
                                0x02 => "Unknown",
                                0x03 => "Instruction",
                                0x04 => "Data",
                                0x05 => "Unified",
                                _ => "Unknown",
                            };
                            event::ProcessorCache {
                                name: format!("L{} {}-Cache", level, type_),
                                size: cache_info.installed_size.into(), // TODO support larger size w/ newer smbios
                            }
                        })
                        .collect();

                    let processor_id = processor.processor_id;
                    events.push(
                        event::Processor {
                            caches: Some(caches),
                            capabilities: None, // XXX
                            cores_count: Some(processor.core_count.into()),
                            cores_enabled: Some(processor.core_enabled.into()),
                            device_id: String::new(), // XXX
                            manufacturer: i.get_str(processor.processor_manufacturer).cloned(),
                            max_clock_speed: Some(i64::from(processor.max_speed)),
                            name: i.get_str(processor.processor_version).cloned(),
                            processor_id: format!("{:X}", processor_id), // XXX: correct?
                            signature: None, // XXX where does dmidecocode get this?
                            socket: i.get_str(processor.socket_designation).cloned(),
                            state: State::Added,
                            thread_count: Some(processor.thread_count.into()),
                            voltage: Some(f64::from(processor.voltage) / 10.),
                        }
                        .into(),
                    );
                }
            }
        }),
        TelemetryEventType::HwDisplay => EventDesc::new(|events| {
            for device in DrmDevice::all() {
                for connector in device.connectors() {
                    if connector.state() != drm::control::connector::State::Connected {
                        continue;
                    }
                    let port = format!("{:?}-{}", connector.interface(), connector.interface_id()); // XXX probably should depend on gpu
                    let pixel_size = device.connector_mode(&connector).map(|mode| mode.size());
                    events.push(
                        event::Display {
                            port,
                            pixel_width: pixel_size.map(|x| x.0 as i64),
                            pixel_height: pixel_size.map(|x| x.1 as i64),
                            state: State::Added,
                        }
                        .into(),
                    );
                }
            }
        }),
        _ => return None,
    })
}

pub fn events_inner<I: Iterator<Item = TelemetryEventType>>(
    types: I,
) -> Vec<event::TelemetryEvent> {
    let mut events = Vec::new();

    let mut udev_descs = UdevDescs::new();
    for i in types {
        match event(i) {
            Some(EventDesc::Periodic(desc)) => {
                desc.generate(&mut events);
            }
            Some(EventDesc::Udev(desc)) => udev_descs.insert(desc),
            None => {}
        }
    }

    // XXX can this ever fail?
    let mut enumerator = udev::Enumerator::new().unwrap();
    for device in enumerator.scan_devices().unwrap() {
        if let Some(subsystem) = device.subsystem().and_then(OsStr::to_str) {
            for desc in udev_descs.get(subsystem) {
                desc.generate(&mut events, &device);
            }
        }
    }

    events
}

pub fn all_events() -> Vec<event::TelemetryEvent> {
    events_inner(event::TelemetryEventType::iter())
}

pub fn events(freqs: &Frequencies, freq: SamplingFrequency) -> Vec<event::TelemetryEvent> {
    events_inner(event::TelemetryEventType::iter().filter(|i| freqs.get(*i) == freq))
}
