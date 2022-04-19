// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::BTreeMap, env, fs, io};

use crate::{api::Api, db::DB, event::DeviceOSIds, util};

use hp_vendor_client::PurposesOutput;

fn api(db: &DB) -> Option<Api> {
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).ok()?;
    Api::new(ids).ok()
}

pub fn purposes() {
    let arg = env::args().skip(1).next();

    util::check_supported_and_create_dir();

    let db = DB::open().unwrap();

    let api = if arg.as_deref() == Some("--no-fetch") {
        None
    } else {
        api(&db)
    };

    let consent = db.get_consent().unwrap();
    let purposes = crate::purposes(&db, api.as_ref());

    serde_json::to_writer(io::stdout(), &PurposesOutput { consent, purposes }).unwrap();
}

pub fn update_purposes() {
    util::check_supported_and_create_dir();

    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).ok().unwrap();
    let api = Api::new(ids).ok().unwrap();

    let purposes = BTreeMap::from_iter(api.purposes(None).unwrap().into_iter());

    let file = fs::File::create("purposes.json").unwrap();
    serde_json::to_writer_pretty(file, &purposes).unwrap();

    eprintln!("Purposes written to `purposes.json`.");
}
