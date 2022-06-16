// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    env,
    io::{self, Write},
    process,
    str::FromStr,
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

fn api(db: &DB) -> Option<Api> {
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).ok()?;
    Api::new(ids).ok()
}

pub fn run(mut args: env::Args) {
    let mut locale = args.next().unwrap_or_else(arg_err);
    let country = args.next().unwrap_or_else(arg_err);
    let purpose_version = if args.len() != 0 {
        let purpose_id = args.next().unwrap_or_else(arg_err);
        let version = args.next().unwrap_or_else(arg_err);
        let opt_in = args
            .next()
            .map_or(true, |arg| bool::from_str(&arg).unwrap());
        Some((purpose_id, version, opt_in))
    } else {
        None
    };

    let db = DB::open().unwrap();

    let consent = if let Some((purpose_id, version, opt_in)) = purpose_version {
        event::DataCollectionConsent {
            country: country,
            locale: locale,
            purpose_id: purpose_id,
            version: version,
            sent: false,
            opt_in,
        }
        // XXX enable or disable
    } else {
        // XXX show existing consent

        let purposes = crate::purposes(&db, api(&db).as_ref());

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

        let opt_in = answer.trim() == "y";

        event::DataCollectionConsent {
            country: country.to_string(),
            locale: locale.to_string(),
            purpose_id: purpose.purpose_id.clone(),
            version: purpose.version.clone(),
            sent: false,
            opt_in,
        }
    };

    db.set_consent(Some(&consent)).unwrap();
    if consent.opt_in {
        util::systemd::disable_opt_out_service();
        util::systemd::enable_services_and_timers();
    } else {
        util::systemd::disable_services_and_timers();
        util::systemd::enable_opt_out_service();
    }
}
