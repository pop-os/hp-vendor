use std::process;

use crate::db::{self, DB};

pub fn run(arg: Option<&str>) {
    let db = DB::open().unwrap();

    match arg {
        Some("consent") => println!("{:#?}", db.get_consent().unwrap()),
        Some("frequencies") => println!("{:#?}", db.get_event_frequencies().unwrap()),
        Some("queued") => println!("{:#?}", db.get_queued(false).unwrap()),
        Some("state") => println!("{:#?}", db.get_state(db::State::All).unwrap()),
        _ => {
            eprintln!("Usage: hp-vendor print (consent|frequencies|queued|state)");
            process::exit(1);
        }
    }
}
