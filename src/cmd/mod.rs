mod consent;
mod daemon;
mod daily;
mod delete;
mod download;
mod print;

use std::{env, fs, io, os::unix::fs::PermissionsExt, process};

fn create_var_dir() -> io::Result<()> {
    fs::create_dir("/var/hp-vendor")?;
    fs::set_permissions("/var/hp-vendor", fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn run() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    if let Err(err) = crate::supported_hardware() {
        eprintln!("Unsupported hardware: {}", err);
        process::exit(1);
    }

    if let Err(err) = create_var_dir() {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Failed to create `/var/hp-vendor`: {}", err);
        }
    }

    let mut args = env::args().skip(1);
    let cmd = args.next();
    let arg = args.next();
    match cmd.as_deref() {
        Some("consent") => consent::run(arg.as_deref()),
        Some("daemon") => daemon::run(),
        Some("daily") => daily::run(arg.as_deref()),
        Some("delete") => delete::run(),
        Some("download") => download::run(arg.as_deref()),
        Some("print") => print::run(arg.as_deref()),
        _ => {
            eprintln!("Usage: hp-vendor (consent|daemon|daily|delete|download|print)");
            process::exit(1);
        }
    }
}
