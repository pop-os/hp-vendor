// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{api::Api, db::DB, event::DeviceOSIds, util};

pub fn run() -> anyhow::Result<()> {
    let db = DB::open()?;
    let os_install_id = db.get_os_install_id()?;
    let ids = DeviceOSIds::new(os_install_id)?;

    let api = Api::new(ids)?;

    api.delete()?;
    util::systemd::disable_services_and_timers();
    db.delete_and_disable()?;

    Ok(())
}
