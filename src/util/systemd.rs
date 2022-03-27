// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process::Command;

/// Restarts daemon if running, to handle frequencies change
pub fn try_restart_daemon() {
    let _ = Command::new("systemctl")
        .args(&["try-restart", "hp-vendor"])
        .status();
}
