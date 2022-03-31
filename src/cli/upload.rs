// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::env;

use crate::{
    api::{self, Api},
    db::DB,
    event, util,
};

pub fn run(mut args: env::Args) {
    let arg = args.next();

    // Get unique lock
    let _lock = util::lock::lock_file_or_panic("/var/hp-vendor/upload.lock");

    // XXX handle db errors?
    let db = DB::open().unwrap();
    crate::exit_if_not_opted_in(&db);

    // Would have exited above if consent is `None`
    let mut consent = db.get_consent().unwrap().unwrap();

    let os_install_id = db.get_os_install_id().unwrap();
    let ids = event::DeviceOSIds::new(os_install_id).unwrap();

    let api = if arg.as_deref() != Some("--dequeue-no-upload") {
        match Api::new(ids.clone()) {
            Ok(api) => Some(api),
            Err(err) => panic!("Failed to authenticate with server: {}", err),
        }
    } else {
        None
    };

    if let Some(api) = &api {
        if !consent.sent {
            let resp = api
                .consent(
                    &consent.locale,
                    &consent.country,
                    &consent.purpose_id,
                    &consent.version,
                )
                .unwrap();
            println!("{:?}", resp);

            consent.sent = true;
            db.set_consent(Some(&consent)).unwrap();
        }

        match api.config() {
            Ok(config) => {
                let frequencies = db.get_event_frequencies().unwrap();
                let new_frequencies = config.frequencies();
                if frequencies != new_frequencies {
                    db.set_event_frequencies(new_frequencies).unwrap();
                    eprintln!("Config changed. Restarting daemon...");
                    util::systemd::try_restart_daemon();
                }
            }
            Err(err) => eprintln!("Error getting frequencies from server: {}", err),
        }
    }

    let (queued_ids, queued) = db.get_queued().unwrap();
    let mut events = event::Events::new(vec![consent], ids, &[]);
    for (chunk_ids, chunk) in queued_ids.chunks(100).zip(queued.chunks(100)) {
        events.data = chunk;

        println!("{}", events.to_json_pretty());

        if let Some(api) = &api {
            let mut start = 0;
            let mut end = chunk.len();
            while start < chunk.len() {
                events.data = &chunk[start..end];
                match api.upload(&events) {
                    Ok(res) => {
                        println!("{:#?}", res);
                        db.remove_queued(&chunk_ids[start..end]).unwrap();
                        start = end;
                        end = chunk.len();
                    }
                    Err(err) => {
                        if err.is::<api::PayloadSizeError>() {
                            // Try to transmit fewer events
                            end = start + (end - start) / 2;
                        } else {
                            panic!("Failed to upload: {}", err);
                        }
                    }
                }
            }
        } else {
            db.remove_queued(chunk_ids).unwrap();
        }
    }
}
