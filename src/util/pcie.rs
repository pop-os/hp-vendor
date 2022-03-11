// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

#![allow(dead_code)]

use std::{collections::HashSet, fmt::Write, path::Path};

const PCI_EXT_CAP_ID_DSN: u16 = 0x03;

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
