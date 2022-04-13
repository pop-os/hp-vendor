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

use std::{env, io, process};

use crate::util;
use hp_vendor_client::{ApiError, ErrorJson};

fn handle_err(res: anyhow::Result<()>) {
    if let Err(err) = res {
        let is_tty = unsafe { libc::isatty(libc::STDERR_FILENO) } == 1;
        if is_tty {
            eprintln!("Error: {}", err);
        } else {
            let error = if let Some(err) = err.downcast_ref::<ApiError>() {
                ErrorJson::Api(err.clone())
            } else {
                ErrorJson::Other(err.to_string())
            };
            serde_json::to_writer(io::stderr(), &error).unwrap();
        }
        process::exit(2);
    }
}

pub fn run() {
    util::check_supported_and_create_dir();

    let mut args = env::args();
    let _ = args.next();
    let cmd = args.next();
    match cmd.as_deref() {
        Some("consent") => consent::run(args),
        Some("daemon") => daemon::run(),
        Some("daily") => daily::run(),
        Some("delete") => handle_err(delete::run()),
        Some("disable") => disable::run(),
        Some("download") => handle_err(download::run(args)),
        Some("exists") => handle_err(exists::run(args)),
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
