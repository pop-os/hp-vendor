// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process;

use crate::{config::SamplingFrequency, db::DB, util};

pub fn run() {
    // Get unique lock
    let _lock = util::lock::lock_file_or_panic("/var/hp-vendor/daily.lock");

    // XXX handle db errors?
    let db = DB::open().unwrap();

    let consents = db.get_consents().unwrap();
    if consents.is_empty() {
        eprintln!("Need to opt-in with `hp-vendor consent``");
        process::exit(0);
    }

    let freqs = db.get_event_frequencies().unwrap();

    crate::update_events_and_queue(&db, &freqs, SamplingFrequency::Daily).unwrap();
    if db.last_weekly_time_expired().unwrap() {
        crate::update_events_and_queue(&db, &freqs, SamplingFrequency::Weekly).unwrap();
        db.update_last_weekly_time().unwrap();
    }
}
