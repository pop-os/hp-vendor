// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{api::Api, db::DB, event::DeviceOSIds};
use std::env;

pub fn run(_ars: env::Args) -> anyhow::Result<()> {
    let db = DB::open()?;
    let os_install_id = db.get_os_install_id()?;
    let ids = DeviceOSIds::new(os_install_id)?;

    let api = Api::new(ids)?;

    print!("{:?}\n", api.exists()?);

    Ok(())
}
