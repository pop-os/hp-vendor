use drm::{control::Device as ControlDevice, Device};
use std::{
    fs,
    os::unix::io::{AsRawFd, RawFd},
    path::Path,
};

pub struct DrmDevice(fs::File);

impl Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

impl DrmDevice {
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

    pub fn connector_mode(
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
