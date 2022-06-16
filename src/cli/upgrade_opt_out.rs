// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

// Informs analytics server of opt-out on upgrade; wasn't sent in earlier
// version of hp-vendor.

use std::fs;

use crate::{db::DB, event, util};

pub fn run() {
    // XXX handle db errors?
    let db = DB::open().unwrap();

    // Test if DB already has explicit opt-in or opt-out
    let consent = db.get_consent().unwrap();
    if consent.is_some() {
        return;
    }

    // Exit if Gnome Initial Setup has not completed, for any user
    let mut gis_done = false;
    for entry in fs::read_dir("/home").unwrap() {
        if entry
            .unwrap()
            .path()
            .join(".config/gnome-initial-setup-done")
            .exists()
        {
            gis_done = true;
            break;
        }
    }
    if !gis_done {
        return;
    }

    // Set explicit opt-out in DB
    let purposes = hp_vendor_client::static_purposes();
    let (locale, country, purpose) = hp_vendor_client::purpose_for_locale(purposes);
    let consent = event::DataCollectionConsent {
        country: country.to_string(),
        locale: locale.to_string(),
        purpose_id: purpose.purpose_id.clone(),
        version: purpose.version.clone(),
        sent: false,
        opt_in: false,
    };
    db.set_consent(Some(&consent)).unwrap();

    // Tell analytics server of opt-out
    util::systemd::enable_opt_out_service();
}
