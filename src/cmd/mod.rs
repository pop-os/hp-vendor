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

    /*
    #[cfg(not(feature = "disable-model-check"))]
    {
        let product_name = std::fs::read_to_string("/sys/class/dmi/id/product_name").ok();
        let product_name = product_name.as_deref().unwrap_or("unknown").trim();
        if product_name != "HP EliteBook 845 G8 Notebook PC" {
            eprintln!("hp-vendor: unknown product '{}'", product_name);
            process::exit(1);
        }
    }
    */

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
