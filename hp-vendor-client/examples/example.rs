// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{fs, io};

fn main() {
    let hp_vendor_client::PurposesOutput {
        purposes, consent, ..
    } = hp_vendor_client::purposes(false).unwrap();
    let purpose = &purposes["en"];
    if consent.is_some() {
        println!("Opted in");
    } else {
        println!("Opted out");
    }
    println!("Statement: {}", purpose.statement);
    println!("Opting in...");
    hp_vendor_client::consent("en", "US", &purpose.purpose_id, &purpose.version).unwrap();
    println!("Downloading to 'hp-vendor-data.json'...");
    let mut file = fs::File::create("hp-vendor-data.json").unwrap();
    let mut download = hp_vendor_client::download(hp_vendor_client::DownloadFormat::Json).unwrap();
    io::copy(&mut download, &mut file).unwrap();
    println!("Disabling...");
    hp_vendor_client::disable().unwrap();
    println!("Deleting...");
    hp_vendor_client::delete_and_disable().unwrap();
}
