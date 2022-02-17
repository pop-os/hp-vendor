use std::process::Command;

// TODO: what should be optional?
#[derive(serde::Deserialize)]
struct SmartLog {
    critical_warning: i64,
    // "temperature"
    avail_spare: i64,
    spare_thresh: i64,
    percent_used: i64,
    // "endurance_grp_critical_warning_summary"
    // "data_units_read"
    // "data_units_written"
    // "host_read_commands"
    // "host_write_commands"
    // "controller_busy_time"
    power_cycles: i64,
    power_on_hours: i64,
    unsafe_shutdowns: i64,
    media_errors: i64,
    num_err_log_entries: i64,
    warning_temp_time: i64,
    // "critical_comp_time"
    // "temperature_sensor_1"
    // "temperature_sensor_2"
    // "thm_temp1_trans_count"
    // "thm_temp2_trans_count"
    // "thm_temp1_total_time"
    // "thm_temp2_total_time"
}

fn smart_log(path: &str) -> Option<SmartLog> {
    let stdout = Command::new("nvme")
        .args(&["smart-log", path, "--output-format=json"])
        .output()
        .ok()?
        .stdout;
    serde_json::from_slice(&stdout).ok()
}
