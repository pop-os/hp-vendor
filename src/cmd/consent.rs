// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::{self, Write},
    process,
};

use crate::{
    api::Api,
    db::DB,
    event::{self, DeviceOSIds},
    util,
};

fn arg_err<'a>() -> &'a str {
    eprintln!("Usage: hp-vendor consent <locale> <country>");
    process::exit(1)
}

pub fn run(arg1: Option<&str>, arg2: Option<&str>) {
    let locale = arg1.unwrap_or_else(arg_err);
    let country = arg2.unwrap_or_else(arg_err);

    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    // XXX show existing consent

    let api = Api::new(ids).unwrap();
    let purposes = api.purposes(locale).unwrap();

    db.set_purposes(locale, &purposes).unwrap();

    let mut consents = Vec::new();
    for purpose in purposes {
        println!("Purpose: {}", purpose.statement);
        consents.push(event::DataCollectionConsent {
            country: country.to_string(),
            locale: purpose.locale,
            purpose_id: purpose.purpose_id,
            version: purpose.version,
            sent: false,
        });
    }

    print!("Agree? [yN]: ");
    io::stdout().lock().flush().unwrap();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();

    if answer.trim() == "y" {
        db.set_opted(Some(true)).unwrap();
        db.set_consents(&consents).unwrap();
        util::systemd::enable_services_and_timers();
    } else {
        db.set_opted(Some(false)).unwrap();
        util::systemd::disable_services_and_timers();
    }
}
