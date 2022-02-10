use drm::{control::Device as ControlDevice, Device};
use nix::sys::utsname::uname;
use os_release::OS_RELEASE;
use plain::Plain;
use std::{
    collections::HashSet,
    fmt::Write,
    fs,
    os::unix::io::{AsRawFd, RawFd},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

pub mod api;
pub mod daemon;
pub mod daily;
mod db;
pub mod event;
pub mod report;
mod util;

use event::{read_file, unknown, State, TelemetryEvent, TelemetryEventType};
use report::ReportFreq;

const PCI_EXT_CAP_ID_DSN: u16 = 0x03;

struct DrmDevice(fs::File);

impl Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

impl DrmDevice {
    fn all() -> impl Iterator<Item = Self> {
        fs::read_dir("/dev/dri")
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.starts_with("card") {
                            return Self::open(entry.path());
                        }
                    }
                }
                None
            })
    }

    fn open<P: AsRef<Path>>(path: P) -> Option<Self> {
        Some(Self(
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .ok()?,
        ))
    }

    fn connectors(&self) -> Vec<drm::control::connector::Info> {
        self.resource_handles()
            .ok()
            .map_or_else(Vec::new, |handles| {
                handles
                    .connectors()
                    .iter()
                    .filter_map(|handle| self.get_connector(*handle).ok())
                    .collect()
            })
    }

    fn connector_mode(
        &self,
        connector: &drm::control::connector::Info,
    ) -> Option<drm::control::Mode> {
        // NOTE: doesn't work on Nvidia without `nvidia-drm.modeset`
        let encoder = self.get_encoder(connector.current_encoder()?).ok()?;
        let crtc = self.get_crtc(encoder.crtc()?).ok()?;
        crtc.mode()
    }
}

impl AsRawFd for DrmDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[repr(packed)]
#[derive(Clone, Default, Debug, Copy)]
#[allow(dead_code)]
struct CacheInfo21 {
    socket: u8,
    configuration: u16,
    maximum_size: u16,
    installed_size: u16,
    supported_sram_type: u16,
    current_sram_type: u16,
    cache_speed: u8,
    error_correction_type: u8,
    system_cache_type: u8,
    associativity: u8,
}

unsafe impl Plain for CacheInfo21 {}

impl dmi::TableKind for CacheInfo21 {
    const KIND: u8 = 7;
}

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

fn battery() -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/class/power_supply").ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if let Ok(type_) = fs::read(path.join("type")) {
            if type_ == b"Battery\n" {
                return Some(path);
            }
        }
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
                    version: utsname.version().to_string(),
                }
                .into(),
            );
        }),
        TelemetryEventType::HwBattery => EventDesc::new(ReportFreq::Daily, |events| {
            let path = match battery() {
                Some(path) => path,
                None => return,
            };

            events.push(
                event::Battery {
                    ct_number: String::new(), // XXX,
                    devicename: read_file(path.join("model_name")),
                    energy_design: read_file(path.join("charge_full_design"))
                        .map(|x: f64| x / 1000000.), // XXX divisor?
                    manufacturer: read_file(path.join("manufacturer")),
                    serial_number: read_file(path.join("serial_number")).unwrap_or_else(unknown),
                    state: State::Added,
                    voltage_design: read_file(path.join("voltage_min_design"))
                        .map(|x: f64| x / 1000.),
                }
                .into(),
            );
        }),
        TelemetryEventType::HwBatteryUsage => EventDesc::new(ReportFreq::Daily, |events| {
            let path = match battery() {
                Some(path) => path,
                None => return,
            };

            // XXX: Division? Integers?
            let energy_rate = || -> Option<i64> {
                let current: i64 = read_file(path.join("current_now"))?;
                let voltage: i64 = read_file(path.join("voltage_now"))?;
                Some(current * voltage / 1000000)
            };
            events.push(
                event::BatteryUsage {
                    battery_state: read_file(path.join("status")).unwrap_or_else(unknown),
                    cell_voltage: None,       // XXX
                    ct_number: String::new(), // XXX
                    cycle_count: read_file(path.join("cycle_count")).unwrap_or(-1),
                    eletric_current: None, // XXX
                    energy_full: read_file(path.join("charge_full"))
                        .map(|x: f64| x / 1000000.)
                        .unwrap_or(-1.),
                    energy_rate: energy_rate(),
                    energy_remaining: read_file(path.join("charge_now"))
                        .map(|x: f64| x / 1000000.)
                        .unwrap_or(-1.),
                    max_error: None, // XXX
                    serial_number: read_file(path.join("serial_number")).unwrap_or_else(unknown),
                    status_register: None, // XXX
                    temperature: None,     // XXX
                    time_to_empty: None,   // XXX
                    timestamp: event::date_time(),
                    voltage: read_file(path.join("voltage_now")).map(|x: i64| x / 1000000),
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
                    state: State::Added,
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
            fn smbios_version() -> Option<String> {
                let entry_point = fs::read("/sys/firmware/dmi/tables/smbios_entry_point").ok()?;
                let smbios = dmi::Smbios::from_bytes(&entry_point).ok()?;
                Some(format!("{}.{}", smbios.major_version, smbios.minor_version))
            }
            events.push(
                event::Firmware {
                    bios_release_date: bios_date(),
                    bios_vendor: read_file("/sys/class/dmi/id/bios_vendor"),
                    bios_version: read_file("/sys/class/dmi/id/bios_version"),
                    capabilities: None, // XXX
                    embedded_controller_version: read_file("/sys/class/dmi/id/ec_firmware_release"),
                    rom_size: None, // XXX
                    smbios_version: smbios_version(),
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
                    state: State::Added,
                    uuid: read_file("/sys/class/dmi/id/product_uuid").unwrap_or_else(unknown),
                    version: read_file("/sys/class/dmi/id/product_version"),
                    width: None, // XXX
                }
                .into(),
            );
        }),
        TelemetryEventType::SwOperatingSystem => EventDesc::new(ReportFreq::Daily, |events| {
            let os_release = OS_RELEASE.as_ref().ok();
            events.push(
                event::OperatingSystem {
                    boot_device: String::new(), // XXX
                    codename: os_release.as_ref().map(|x| x.version_codename.to_owned()),
                    manufacturer: None, // XXX
                    name: os_release
                        .as_ref()
                        .map_or_else(unknown, |x| x.name.to_owned()),
                    sku: None, // XXX
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
                    let modinfo = |field| {
                        let res = Command::new("/usr/sbin/modinfo")
                            .args(["-F", field, module_name])
                            .output()
                            .ok()?;
                        if !res.status.success() {
                            return None;
                        }
                        let mut s = String::from_utf8(res.stdout).ok()?;
                        s.truncate(s.trim_end().len());
                        Some(s)
                    };
                    events.push(
                        event::Driver {
                            author: None,                       // XXX
                            description: None,                  // XXX
                            driver_version: modinfo("version"), // XXX most don't have version
                            link_time: None,                    // XXX
                            module_name: module_name.to_string(),
                            module_path: String::new(), // XXX
                            module_type: String::new(), // XXX
                            size,
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
                        state: State::Added,
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
        TelemetryEventType::HwPeripheralUsbTypeA => EventDesc::new(ReportFreq::Trigger, |_| {}),
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
                            state: State::Added,
                            type_: Some(type_),
                        }
                        .into(),
                    )
                }
            }
        }),
        TelemetryEventType::HwProcessor => EventDesc::new(ReportFreq::Daily, |events| {
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
                            max_clock_speed: Some(
                                (u64::from(processor.max_speed) * 1000).to_string(),
                            ), // XXX why string?
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
        TelemetryEventType::HwDisplay => EventDesc::new(ReportFreq::Daily, |events| {
            for device in DrmDevice::all() {
                for connector in device.connectors() {
                    let connected = connector.state() == drm::control::connector::State::Connected;
                    let display_name =
                        format!("{:?}{}", connector.interface(), connector.interface_id()); // XXX probably should depend on gpu
                    let pixel_size = device.connector_mode(&connector).map_or(0, |mode| {
                        let (width, height) = mode.size();
                        width as i64 * height as i64
                    }); // XXX ?
                    events.push(
                        event::Display {
                            connected,
                            display_name,
                            pixel_size,
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

pub fn all_events() -> Vec<event::TelemetryEvent> {
    let mut events = Vec::new();
    for i in event::TelemetryEventType::iter() {
        if let Some(event) = event(i) {
            event.generate(&mut events);
        }
    }
    events
}

pub fn peripheral_usb_type_a_event(path: &Path) -> Option<TelemetryEvent> {
    if !path.join("idProduct").exists() {
        return None;
    }

    Some(
        event::PeripheralUSBTypeA {
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
        }
        .into(),
    )
}
