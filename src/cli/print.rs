// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{env, process};

use crate::{
    api::Api,
    db::{self, DB},
    event::DeviceOSIds,
};

fn api(db: &DB) -> Option<Api> {
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).ok()?;
    Api::new(ids).ok()
}

pub fn run(mut args: env::Args) {
    let db = DB::open().unwrap();

    match args.next().as_deref() {
        Some("consent") => println!("{:#?}", db.get_consent().unwrap()),
        Some("frequencies") => println!("{:#?}", db.get_event_frequencies().unwrap()),
        Some("purposes") => println!("{:#?}", crate::purposes(&db, api(&db).as_ref())),
        Some("queued") => println!("{:#?}", db.get_queued().unwrap().1),
        Some("state") => println!("{:#?}", db.get_state(db::State::All).unwrap()),
        _ => {
            eprintln!("Usage: hp-vendor print (consent|frequencies|purposes|queued|state)");
            process::exit(1);
        }
    }
}
