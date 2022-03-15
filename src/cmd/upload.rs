// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process;

use crate::{
    api::{self, Api},
    db::DB,
    event, util,
};

pub fn run(arg: Option<&str>) {
    // Get unique lock
    let _lock = util::lock_file_or_panic("/var/hp-vendor/daily-upload.lock");

    // XXX handle db errors?
    let db = DB::open().unwrap();

    let consents = db.get_consents().unwrap();
    if consents.is_empty() {
        eprintln!("Need to opt-in with `hp-vendor consent``");
        process::exit(0);
    }
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = event::DeviceOSIds::new(os_install_id).unwrap();

    let api = if arg != Some("--dequeue-no-upload") {
        match Api::new(ids.clone()) {
            Ok(api) => Some(api),
            Err(err) => panic!("Failed to authenticate with server: {}", err),
        }
    } else {
        None
    };

    if let Some(api) = &api {
        // XXX try to get config before daily sampling
        // XXX restart daemon if changed
        match api.config() {
            Ok(config) => db.set_event_frequencies(config.frequencies()).unwrap(),
            Err(err) => eprintln!("Error getting frequencies from server: {}", err),
        }
    }

    let (queued_ids, queued) = db.get_queued().unwrap();
    let mut events = event::Events::new(consents, ids, &[]);
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
