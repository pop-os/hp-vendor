use nix::errno::Errno;
use std::{fs, io, os::unix::fs::PermissionsExt};

use crate::{
    all_events,
    event::{self, TelemetryEvent},
    util,
};

pub fn run() {
    if let Err(err) = fs::create_dir("/var/hp-vendor") {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Failed to create `/var/hp-vendor`: {}", err);
        }
    }

    // Get unique lock
    let lock_file = fs::File::create("/var/hp-vendor/lock").unwrap();
    if let Err(err) = util::setlk(&lock_file) {
        if err == Errno::EACCES || err == Errno::EAGAIN {
            panic!("Lock already held on `/var/hp-vendor/lock`");
        } else {
            panic!("Error locking `/var/hp-vendor/lock`: {}", err);
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
    new_file
        .set_permissions(fs::Permissions::from_mode(0o600))
        .unwrap();
    serde_json::to_writer(new_file, &new).unwrap();

    if let Some(old) = old {
        event::diff(&mut new, &old);
    }

    let events = event::Events::new(new);
    println!("{}", events.to_json_pretty());

    /*
    let client = reqwest::blocking::Client::new();
    let token = hp_vendor::api::TokenRequest::new()
        .send(&client)
        .unwrap()
        .token;
    println!("{:#?}", events.send(&client, &token).unwrap());
    */

    fs::rename(
        "/var/hp-vendor/daily-updated.json",
        "/var/hp-vendor/daily.json",
    )
    .unwrap();
}
