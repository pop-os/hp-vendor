// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{env, process};

use crate::db::{self, DB};

pub fn run(mut args: env::Args) {
    let db = DB::open().unwrap();

    match args.next().as_deref() {
        Some("consent") => println!("{:#?}", db.get_consent().unwrap()),
        Some("frequencies") => println!("{:#?}", db.get_event_frequencies().unwrap()),
        Some("purposes") => println!("{:#?}", crate::purposes()),
        Some("queued") => println!("{:#?}", db.get_queued().unwrap().1),
        Some("state") => println!("{:#?}", db.get_state(db::State::All).unwrap()),
        _ => {
            eprintln!("Usage: hp-vendor print (consent|frequencies|purposes|queued|state)");
            process::exit(1);
        }
    }
}
