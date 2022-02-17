use plain::Plain;
use std::fs;

pub fn dmi() -> Vec<dmi::Table> {
    if let Ok(data) = fs::read("/sys/firmware/dmi/tables/DMI") {
        dmi::tables(&data)
    } else {
        Vec::new()
    }
}

#[repr(packed)]
#[derive(Clone, Default, Debug, Copy)]
#[allow(dead_code)]
pub struct CacheInfo21 {
    pub socket: u8,
    pub configuration: u16,
    pub maximum_size: u16,
    pub installed_size: u16,
    pub supported_sram_type: u16,
    pub current_sram_type: u16,
    pub cache_speed: u8,
    pub error_correction_type: u8,
    pub system_cache_type: u8,
    pub associativity: u8,
}

unsafe impl Plain for CacheInfo21 {}

impl dmi::TableKind for CacheInfo21 {
    const KIND: u8 = 7;
}

#[repr(packed)]
#[derive(Clone, Default, Debug, Copy)]
#[allow(dead_code)]
struct SystemInfo21 {
    pub manufacturer: u8,
    pub name: u8,
    pub version: u8,
    pub serial: u8,
    pub uuid: u128,
    pub wake_up_type: u8,
    // SMBIOS 2.4?
    // sku: u8,
    // family: u8,
}

unsafe impl Plain for SystemInfo21 {}

impl dmi::TableKind for SystemInfo21 {
    const KIND: u8 = 1;
}
