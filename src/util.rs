// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{fs, io, os::unix::fs::PermissionsExt, process};

pub mod dmi;
pub mod drm;
pub mod lock;
pub mod nvme;
pub mod pcie;
pub mod sensors;
pub mod systemd;

pub use hp_vendor_client::conf::{hp_vendor_conf, HpVendorConf};

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

    if let Err(err) = hp_vendor_client::supported_hardware() {
        eprintln!("Unsupported hardware: {}", err);
        process::exit(1);
    }

    if let Err(err) = create_var_dir() {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Failed to create `/var/hp-vendor`: {}", err);
        }
    }
}
