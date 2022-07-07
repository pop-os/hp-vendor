// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, process::Command};

#[derive(Debug)]
pub struct ControllerId {
    pub sn: String,
    // mn: String,
    pub ver: i64,
    pub wctemp: i64,
    pub cctemp: i64,
    // Ignoring fields that aren't useful
}

impl ControllerId {
    pub fn ver(&self) -> String {
        let major = self.ver >> 16;
        let minor = (self.ver >> 8) & 0xff;
        format!("{}.{}", major, minor)
    }

    // Json output of `nvme` has issues: https://github.com/linux-nvme/nvme-cli/issues/1598
    // Parse binary instead.
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let sn = bytes.get(04..=23)?;
        let sn = String::from_utf8_lossy(sn).into_owned();

        let ver = bytes.get(80..=83)?;
        let ver = i64::from(u32::from_le_bytes([ver[0], ver[1], ver[2], ver[3]]));

        let wctemp = bytes.get(266..=267)?;
        let wctemp = i64::from(u16::from_le_bytes([wctemp[0], wctemp[1]]));

        let cctemp = bytes.get(268..=269)?;
        let cctemp = i64::from(u16::from_le_bytes([cctemp[0], cctemp[1]]));

        Some(Self {
            sn,
            ver,
            wctemp,
            cctemp,
        })
    }
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct NamespaceId {
    nuse: i64,
    // Ignoring fields that aren't useful
}

// TODO: what should be optional?
// For parsing JSON output of `nvme smart-log`
// See also `struct nvme_smart_log`
#[derive(serde::Deserialize)]
pub struct SmartLog {
    pub critical_warning: i64,
    // "temperature"
    pub avail_spare: i64,
    pub spare_thresh: i64,
    pub percent_used: i64,
    pub endurance_grp_critical_warning_summary: i64,
    pub data_units_read: u128,
    pub data_units_written: u128,
    pub host_read_commands: u128,
    pub host_write_commands: i128,
    pub controller_busy_time: u128,
    pub power_cycles: u128,
    pub power_on_hours: u128,
    pub unsafe_shutdowns: u128,
    pub media_errors: u128,
    pub num_err_log_entries: u128,
    pub warning_temp_time: i64,
    pub critical_comp_time: i64,
    pub temperature_sensor_1: Option<i64>,
    pub temperature_sensor_2: Option<i64>,
    pub temperature_sensor_3: Option<i64>,
    pub temperature_sensor_4: Option<i64>,
    pub temperature_sensor_5: Option<i64>,
    pub temperature_sensor_6: Option<i64>,
    pub temperature_sensor_7: Option<i64>,
    pub temperature_sensor_8: Option<i64>,
    pub thm_temp1_trans_count: i64,
    pub thm_temp2_trans_count: i64,
    pub thm_temp1_total_time: i64,
    pub thm_temp2_total_time: i64,
}

impl SmartLog {
    pub fn temperature_sensors(&self) -> Vec<i64> {
        [
            self.temperature_sensor_1,
            self.temperature_sensor_2,
            self.temperature_sensor_3,
            self.temperature_sensor_4,
            self.temperature_sensor_5,
            self.temperature_sensor_6,
            self.temperature_sensor_7,
            self.temperature_sensor_8,
        ]
        .iter()
        .filter_map(|x| *x)
        .collect()
    }
}

fn nvme_cmd_binary<S: AsRef<OsStr>>(cmd: &str, path: S) -> Option<Vec<u8>> {
    Some(
        Command::new("nvme")
            .arg(cmd)
            .arg(&path)
            .arg("--output-format=binary")
            .output()
            .ok()?
            .stdout,
    )
}

fn nvme_cmd<S: AsRef<OsStr>, T: serde::de::DeserializeOwned>(cmd: &str, path: S) -> Option<T> {
    let stdout = Command::new("nvme")
        .arg(cmd)
        .arg(&path)
        .arg("--output-format=json")
        .output()
        .ok()?
        .stdout;
    serde_json::from_slice(&stdout).ok()
}

pub fn smart_log<S: AsRef<OsStr>>(path: S) -> Option<SmartLog> {
    nvme_cmd("smart-log", path)
}

pub fn controller_id<S: AsRef<OsStr>>(path: S) -> Option<ControllerId> {
    ControllerId::from_bytes(&nvme_cmd_binary("id-ctrl", path)?)
}

#[allow(dead_code)]
pub fn namespace_id<S: AsRef<OsStr>>(path: S) -> Option<NamespaceId> {
    nvme_cmd("id-ns", path)
}
