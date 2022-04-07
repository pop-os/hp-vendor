// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    api::{Api, DownloadFormat},
    db::DB,
    event::DeviceOSIds,
};
use std::{env, io, str::FromStr};

pub fn run(mut arg: env::Args) {
    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    let api = Api::new(ids).unwrap();

    let format = arg
        .next()
        .map(|s| DownloadFormat::from_str(&s).expect("Invalid format"))
        .unwrap_or(DownloadFormat::Json);
    let mut res = api.download(format).unwrap();
    io::copy(&mut res, &mut io::stdout().lock()).unwrap();
}
