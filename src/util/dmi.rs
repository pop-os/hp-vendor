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
pub struct DmiUuid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}

impl From<&DmiUuid> for uuid::Uuid {
    fn from(uuid: &DmiUuid) -> Self {
        Self::from_fields(uuid.d1, uuid.d2, uuid.d3, &uuid.d4)
            .ok()
            .unwrap_or_else(Self::nil)
    }
}

#[repr(packed)]
#[derive(Clone, Default, Debug, Copy)]
#[allow(dead_code)]
pub struct SystemInfo24 {
    pub manufacturer: u8,
    pub name: u8,
    pub version: u8,
    pub serial: u8,
    pub uuid: DmiUuid,
    pub wake_up_type: u8,
    pub sku: u8,
    pub family: u8,
}

unsafe impl Plain for SystemInfo24 {}

impl dmi::TableKind for SystemInfo24 {
    const KIND: u8 = 1;
}

#[repr(packed)]
#[derive(Clone, Default, Debug, Copy)]
#[allow(dead_code)]
pub struct BiosInfo31 {
    pub vendor: u8,
    pub version: u8,
    pub address: u16,
    pub date: u8,
    pub rom_size: u8,
    pub characteristics: u64,
    pub characteristics_extension_bytes: [u8; 2],
    pub system_major: u8,
    pub system_minor: u8,
    pub ec_major: u8,
    pub ec_minor: u8,
    pub extended_rom_size: u16,
}

unsafe impl Plain for BiosInfo31 {}

impl dmi::TableKind for BiosInfo31 {
    const KIND: u8 = 0;
}
