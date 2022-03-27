// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

#![allow(dead_code)]

use std::process::Command;

const SERVICE: &str = "hp-vendor.service";
const SERVICES_AND_TIMERS: &[&str] = &[SERVICE, "hp-vendor-daily.timer", "hp-vendor-upload.timer"];

/// Restarts daemon if running, to handle frequencies change
pub fn try_restart_daemon() {
    let _ = Command::new("systemctl")
        .args(&["try-restart", SERVICE])
        .status();
}

pub fn enable_services_and_timers() {
    let _ = Command::new("systemctl")
        .arg("enable")
        .args(SERVICES_AND_TIMERS)
        .status();
    let _ = Command::new("systemctl")
        .arg("start")
        .args(SERVICES_AND_TIMERS)
        .status();
}

pub fn disable_services_and_timers() {
    let _ = Command::new("systemctl")
        .arg("stop")
        .args(SERVICES_AND_TIMERS)
        .status();
    let _ = Command::new("systemctl")
        .arg("disable")
        .args(SERVICES_AND_TIMERS)
        .status();
}
