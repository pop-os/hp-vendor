// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{api::Api, db::DB, event, util};

pub fn run() {
    // XXX handle db errors?
    let db = DB::open().unwrap();

    let mut consent = db.get_consent().unwrap().unwrap();

    let os_install_id = db.get_os_install_id().unwrap();
    let ids = event::DeviceOSIds::new(os_install_id).unwrap();

    let api = match Api::new(ids.clone()) {
        Ok(api) => api,
        Err(err) => panic!("Failed to authenticate with server: {}", err),
    };

    if !consent.sent && !consent.opt_in {
        let resp = api
            .consent(
                false,
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

    util::systemd::disable_opt_out_service();
}
