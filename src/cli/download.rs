// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    api::{Api, DownloadFormat},
    db::DB,
    event::DeviceOSIds,
    io::Write,
};
use std::{env, io, str::FromStr};

pub fn run(mut args: env::Args) -> anyhow::Result<()> {
    let db = DB::open()?;
    let os_install_id = db.get_os_install_id()?;
    let ids = DeviceOSIds::new(os_install_id)?;

    let api = Api::new(ids)?;

    let format = args
        .next()
        .map(|s| DownloadFormat::from_str(&s).expect("Invalid format"))
        .unwrap_or(DownloadFormat::Json);
    let (length, mut data) = api.download(format)?;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if args.next().as_deref() == Some("--binary-content-length") {
        stdout.write_all(&u64::to_le_bytes(length))?;
    }
    io::copy(&mut data, &mut stdout)?;

    Ok(())
}
