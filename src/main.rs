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

    let mut events = Vec::new();
    for i in event::TelemetryEventType::iter() {
        if let Some(event) = hp_vendor::event(i) {
            event.generate(&mut events);
        }
    }
    println!("{}", event::Event::new(events).to_json_pretty());
}
