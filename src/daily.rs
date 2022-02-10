use nix::errno::Errno;
use std::fs;

use crate::{all_events, db::DB, event, util};

pub fn run() {
    // Get unique lock
    let lock_file = fs::File::create("/var/hp-vendor/lock").unwrap();
    if let Err(err) = util::setlk(&lock_file) {
        if err == Errno::EACCES || err == Errno::EAGAIN {
            panic!("Lock already held on `/var/hp-vendor/lock`");
        } else {
            panic!("Error locking `/var/hp-vendor/lock`: {}", err);
        }
    }

    // XXX handle db errors?
    let db = DB::open().unwrap();
    db.update_event_types().unwrap();

    // TODO: handle frequencies other than daily
    let old = db.get_state_with_freq("daily").unwrap();

    // TODO: only handle daily events, etc.
    let new = all_events();
    let mut diff = new.clone();
    event::diff(&mut diff, &old);

    let events = event::Events::new(diff);
    println!("{}", events.to_json_pretty());

    /*
    let client = reqwest::blocking::Client::new();
    let token = hp_vendor::api::TokenRequest::new()
        .send(&client)
        .unwrap()
        .token;
    println!("{:#?}", events.send(&client, &token).unwrap());
    */

    db.replace_state_with_freq("daily", &new).unwrap();
}
