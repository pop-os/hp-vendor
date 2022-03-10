use std::{ffi::OsStr, process::Command};

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
        ]
        .iter()
        .filter_map(|x| *x)
        .collect()
    }
}

pub fn smart_log<S: AsRef<OsStr>>(path: S) -> Option<SmartLog> {
    let stdout = Command::new("nvme")
        .arg("smart-log")
        .arg(&path)
        .arg("--output-format=json")
        .output()
        .ok()?
        .stdout;
    Some(serde_json::from_slice(&stdout).unwrap())
}
