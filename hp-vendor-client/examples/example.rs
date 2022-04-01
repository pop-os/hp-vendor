// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::fs;

fn main() {
    let hp_vendor_client::PurposesOutput {
        purposes, consent, ..
    } = hp_vendor_client::purposes().unwrap();
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
    hp_vendor_client::download(
        fs::File::create("hp-vendor-data.json").unwrap(),
        hp_vendor_client::DownloadFormat::Json,
    )
    .unwrap();
    println!("Disabling...");
    hp_vendor_client::disable().unwrap();
    println!("Deleting...");
    hp_vendor_client::delete_and_disable().unwrap();
}
