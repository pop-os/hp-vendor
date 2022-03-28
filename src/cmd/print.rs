// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process;

use crate::db::{self, DB};

pub fn run(arg: Option<&str>) {
    let db = DB::open().unwrap();

    match arg {
        Some("consents") => println!("{:#?}", db.get_consents().unwrap()),
        Some("frequencies") => println!("{:#?}", db.get_event_frequencies().unwrap()),
        Some("queued") => println!("{:#?}", db.get_queued().unwrap().1),
        Some("state") => println!("{:#?}", db.get_state(db::State::All).unwrap()),
        _ => {
            eprintln!("Usage: hp-vendor print (consent|frequencies|queued|state)");
            process::exit(1);
        }
    }
}
