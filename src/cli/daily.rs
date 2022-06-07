// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{config::SamplingFrequency, db::DB, util};

pub fn run() {
    // Get unique lock
    let _lock = util::lock::lock_file_or_panic("/var/hp-vendor/daily.lock");

    // XXX handle db errors?
    let db = DB::open().unwrap();
    crate::exit_if_not_opted_in(&db);

    let freqs = db.get_event_frequencies().unwrap();

    crate::update_events_and_queue(&db, &freqs, SamplingFrequency::Daily).unwrap();
    if db.last_weekly_time_expired().unwrap() {
        crate::update_events_and_queue(&db, &freqs, SamplingFrequency::Weekly).unwrap();
        db.update_last_weekly_time().unwrap();
    }

    let mut insert_statement = db.prepare_queue_insert().unwrap();
    loop {
        let temps = db.get_temps(true).unwrap();
        if temps.len() < 100 {
            break;
        }
        insert_statement
            .execute(&util::sumarize_temps(&temps).into())
            .unwrap();
        if let Some(battery_life) = util::sumarize_battery_life(&temps) {
            insert_statement.execute(&battery_life.into()).unwrap();
        }
        db.remove_temps_before(temps.last().unwrap()).unwrap();
    }
}
