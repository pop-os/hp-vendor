// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use once_cell::sync::Lazy;
use std::{fs, io, os::unix::fs::PermissionsExt, process};

pub mod dmi;
pub mod drm;
pub mod lock;
pub mod nvme;
pub mod pcie;
pub mod sensors;
pub mod systemd;

const CONF_PATH: &str = "/etc/hp-vendor.conf";
const DEFAULT_ENDPOINT_URL: &str = "https://api.data.hpdevone.com";

#[derive(Default, serde::Deserialize)]
pub struct HpVendorConf {
    endpoint_url: Option<String>,
    #[serde(default)]
    pub allow_unsupported_hardware: bool,
}

impl HpVendorConf {
    pub fn endpoint_url(&self) -> &str {
        self.endpoint_url.as_deref().unwrap_or(DEFAULT_ENDPOINT_URL)
    }
}

pub fn hp_vendor_conf() -> &'static HpVendorConf {
    static CONF: Lazy<HpVendorConf> = Lazy::new(|| {
        let bytes = match fs::read(CONF_PATH) {
            Ok(bytes) => bytes,
            Err(_) => {
                return HpVendorConf::default();
            }
        };
        toml::from_slice(&bytes).unwrap_or_else(|err| {
            eprintln!("Failed to parse `{}`: {}", CONF_PATH, err);
            HpVendorConf::default()
        })
    });
    &CONF
}

fn create_var_dir() -> io::Result<()> {
    fs::create_dir("/var/hp-vendor")?;
    fs::set_permissions("/var/hp-vendor", fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn check_supported_and_create_dir() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    if let Err(err) = crate::supported_hardware() {
        eprintln!("Unsupported hardware: {}", err);
        process::exit(1);
    }

    if let Err(err) = create_var_dir() {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Failed to create `/var/hp-vendor`: {}", err);
        }
    }
}
