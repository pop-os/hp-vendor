// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    env,
    io::{self, Write},
    process,
};

use crate::{
    api::Api,
    db::DB,
    event::{self, DeviceOSIds},
    util,
};

fn arg_err<'a>() -> String {
    eprintln!("Usage: hp-vendor consent <locale> <country> [purpose_id version]");
    process::exit(1)
}

pub fn run(mut args: env::Args) {
    let mut locale = args.next().unwrap_or_else(arg_err);
    let country = args.next().unwrap_or_else(arg_err);
    let purpose_version = if args.len() != 0 {
        let purpose_id = args.next().unwrap_or_else(arg_err);
        let version = args.next().unwrap_or_else(arg_err);
        Some((purpose_id, version))
    } else {
        None
    };

    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    let consent = if let Some((purpose_id, version)) = purpose_version {
        event::DataCollectionConsent {
            country: country,
            locale: locale,
            purpose_id: purpose_id,
            version: version,
            sent: false,
        }
    } else {
        // XXX show existing consent

        let api = Api::new(ids).unwrap();
        let purposes = api.purposes(None).unwrap();
        db.set_purposes(&purposes).unwrap();

        let purpose = if let Some(purpose) = purposes.get(&locale) {
            purpose
        } else {
            locale = "en".to_string();
            &purposes["en"]
        };

        println!("{}", purpose.statement);
        print!("Agree? [yN]: ");
        io::stdout().lock().flush().unwrap();
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();

        if answer.trim() != "y" {
            return;
        }

        event::DataCollectionConsent {
            country: country.to_string(),
            locale: locale.to_string(),
            purpose_id: purpose.purpose_id.clone(),
            version: purpose.version.clone(),
            sent: false,
        }
    };

    db.set_opted(Some(true)).unwrap();
    db.set_consent(Some(&consent)).unwrap();
    util::systemd::enable_services_and_timers();
}
