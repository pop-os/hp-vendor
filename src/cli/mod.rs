// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

mod consent;
mod daemon;
mod daily;
mod delete;
mod disable;
mod download;
mod exists;
mod print;
mod upload;

use std::{env, process};

use crate::util;

pub fn run() {
    util::check_supported_and_create_dir();

    let mut args = env::args();
    let _ = args.next();
    let cmd = args.next();
    match cmd.as_deref() {
        Some("consent") => consent::run(args),
        Some("daemon") => daemon::run(),
        Some("daily") => daily::run(),
        Some("delete") => delete::run(),
        Some("disable") => disable::run(),
        Some("download") => download::run(args),
        Some("exists") => exists::run(args),
        Some("print") => print::run(args),
        Some("upload") => upload::run(args),
        _ => {
            eprintln!(
                "Usage: hp-vendor (consent|daemon|daily|delete|disable|download|exists|print|upload)"
            );
            process::exit(1);
        }
    }
}
