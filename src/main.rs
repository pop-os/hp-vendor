use std::{fs, process};

use hp_vendor::event;

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    let product_name = fs::read_to_string("/sys/class/dmi/id/product_name").unwrap();
    #[cfg(not(feature = "disable-model-check"))]
    if product_name != "HP EliteBook 845 G8 Notebook PC" {
        eprintln!("hp-vendor: unknown product '{}'", product_name);
        process::exit(1);
    }

    println!(
        "{}",
        event::Event::new(
            event::TelemetryEventType::iter()
                .filter_map(hp_vendor::event)
                .map(|x| x.generate())
                .collect()
        )
        .to_json_pretty()
    );
}
