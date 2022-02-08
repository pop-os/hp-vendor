use std::{env, process};

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    #[cfg(not(feature = "disable-model-check"))]
    {
        let product_name = std::fs::read_to_string("/sys/class/dmi/id/product_name").ok();
        let product_name = product_name.as_deref().unwrap_or("unknown").trim();
        if product_name != "HP EliteBook 845 G8 Notebook PC" {
            eprintln!("hp-vendor: unknown product '{}'", product_name);
            process::exit(1);
        }
    }

    match env::args().nth(1).as_deref() {
        Some("daemon") => hp_vendor::daemon::run(),
        Some("daily") => hp_vendor::daily::run(),
        _ => {
            eprintln!("Usage: hp-vendor (daemon|daily)");
            process::exit(1);
        }
    }
}
