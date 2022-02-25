use std::{ffi::OsStr, process::Command};

// TODO: what should be optional?
#[derive(serde::Deserialize)]
pub struct SmartLog {
    pub critical_warning: i64,
    // "temperature"
    pub avail_spare: i64,
    pub spare_thresh: i64,
    pub percent_used: i64,
    // "endurance_grp_critical_warning_summary"
    pub data_units_read: i64,
    pub data_units_written: i64,
    pub host_read_commands: i64,
    pub host_write_commands: i64,
    pub controller_busy_time: i64,
    pub power_cycles: i64,
    pub power_on_hours: i64,
    pub unsafe_shutdowns: i64,
    // pub media_errors: i64,
    pub num_err_log_entries: i64,
    pub warning_temp_time: i64,
    pub critical_comp_time: i64,
    // "temperature_sensor_1"
    // "temperature_sensor_2"
    // "thm_temp1_trans_count"
    // "thm_temp2_trans_count"
    // "thm_temp1_total_time"
    // "thm_temp2_total_time"
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
