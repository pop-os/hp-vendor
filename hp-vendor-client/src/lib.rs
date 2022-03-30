// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::process::Command;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DataCollectionPurpose {
    pub locale: String,
    pub purpose_id: String,
    pub version: String,
    pub min_version: String,
    pub statement: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PurposesOutput {
    pub opted: Option<bool>,
    pub purposes: Option<Vec<DataCollectionPurpose>>,
}

// XXX return error
pub fn purposes(locale: &str) -> PurposesOutput {
    let output = Command::new("pkexec")
        .args(&["/usr/libexec/hp-vendor-purposes", locale])
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}
