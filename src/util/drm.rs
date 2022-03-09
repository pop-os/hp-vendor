use drm::{
    control::{Device as ControlDevice, ModeTypeFlags},
    Device,
};
use plain::Plain;
use std::{
    fs,
    os::unix::io::{AsRawFd, RawFd},
    path::Path,
};

pub struct DrmDevice(fs::File);

impl Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

impl DrmDevice {
    #[allow(dead_code)]
    pub fn all() -> impl Iterator<Item = Self> {
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

    pub fn open<P: AsRef<Path>>(path: P) -> Option<Self> {
        Some(Self(
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .ok()?,
        ))
    }

    pub fn connectors(&self) -> Vec<drm::control::connector::Info> {
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

    #[allow(dead_code)]
    pub fn connector_mode(
        &self,
        connector: &drm::control::connector::Info,
    ) -> Option<drm::control::Mode> {
        // NOTE: doesn't work on Nvidia without `nvidia-drm.modeset`
        let encoder = self.get_encoder(connector.current_encoder()?).ok()?;
        let crtc = self.get_crtc(encoder.crtc()?).ok()?;
        crtc.mode()
    }

    pub fn connector_preferred_mode(
        &self,
        connector: &drm::control::connector::Info,
    ) -> Option<drm::control::Mode> {
        connector
            .modes()
            .iter()
            .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
            .copied()
    }

    #[allow(dead_code)]
    pub fn connector_edid(&self, connector: &drm::control::connector::Info) -> Option<EDIDHeader> {
        let properties = self.get_properties(connector.handle()).ok()?;
        let (handles, values) = properties.as_props_and_values();
        for (handle, raw_value) in handles.iter().zip(values) {
            let prop = self.get_property(*handle).ok()?;
            if prop.name().to_bytes() == b"EDID" {
                let bytes = self.get_property_blob(*raw_value).ok()?;
                let mut header = EDIDHeader::default();
                plain::copy_from_bytes(&mut header, &bytes).ok()?;
                return Some(header);
            }
        }
        None
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
pub struct EDIDHeader {
    pub magic: [u8; 8],
    pub manufacturer: [u8; 2], // encodes 3 5-bit letters
    pub product_code: u16,     // little endian
    pub serial_number: u32,    // little endian
    pub week: u8,
    pub year: u8,
    pub edid_version: u8,
    pub edid_revision: u8,
}

unsafe impl Plain for EDIDHeader {}
