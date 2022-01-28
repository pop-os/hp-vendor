use std::{env, fs, io, process};

use hp_vendor::{
    all_events,
    event::{self, TelemetryEvent},
};

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

    // env::args().nth(1);

    if let Err(err) = fs::create_dir("/var/hp-vendor") {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Failed to create `/var/hp-vendor`: {}", err);
        }
    }

    let old: Option<Vec<TelemetryEvent>> = match fs::File::open("/var/hp-vendor/daily.json") {
        Ok(file) => Some(serde_json::from_reader(file).unwrap()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            panic!("Failed to open `/var/hp-vendor/daily.json`: {}", err);
        }
    };

    // TODO: only handle daily events, etc.
    let mut new = all_events();

    let new_file = fs::File::create("/var/hp-vendor/daily-updated.json").unwrap();
    serde_json::to_writer(new_file, &new).unwrap();

    if let Some(old) = old {
        event::diff(&mut new, &old);
    }

    let events = event::Events::new(new);
    println!("{}", events.to_json_pretty());

    /*
    let client = reqwest::blocking::Client::new();
    let req = hp_vendor::api::TokenRequest {
        devicesn: "a".to_string(),
        biosuuid: "aa".to_string(),
    };
    let token = req.send(&client).unwrap().token;
    println!("{:#?}", events.send(&client, &token).unwrap());
    */

    fs::rename(
        "/var/hp-vendor/daily-updated.json",
        "/var/hp-vendor/daily.json",
    )
    .unwrap();
}
