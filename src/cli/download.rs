// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    api::{Api, DownloadFormat},
    db::DB,
    event::DeviceOSIds,
};
use std::{
    io::{self, Write},
    str::FromStr,
};

pub fn run(arg: Option<&str>) {
    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    let api = Api::new(ids).unwrap();

    let format = arg
        .map(|s| DownloadFormat::from_str(s).expect("Invalid format"))
        .unwrap_or(DownloadFormat::Json);
    let res = api.download(format).unwrap();
    io::stdout().write_all(&res).unwrap();
}
