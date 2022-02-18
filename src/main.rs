use std::{env, fs, io, os::unix::fs::PermissionsExt, process};

fn create_var_dir() -> io::Result<()> {
    fs::create_dir("/var/hp-vendor")?;
    fs::set_permissions("/var/hp-vendor", fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn main() {
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
    match args.next().as_deref() {
        Some("daemon") => hp_vendor::daemon::run(),
        Some("daily") => hp_vendor::daily::run(args.next().as_deref()),
        Some("delete") => hp_vendor::delete::run(),
        Some("download") => hp_vendor::download::run(args.next().as_deref()),
        Some("print") => hp_vendor::print::run(args.next().as_deref()),
        _ => {
            eprintln!("Usage: hp-vendor (daemon|daily|delete|download|print)");
            process::exit(1);
        }
    }
}
