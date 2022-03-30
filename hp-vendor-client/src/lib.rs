// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{fmt, io, process::Command};

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    Io(io::Error),
    PkexecNoauth,
    PkexecDismissed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SerdeJson(err) => write!(f, "{}", err),
            Self::Io(err) => write!(f, "{}", err),
            Self::PkexecNoauth => write!(f, "Polkit authorization failed"),
            Self::PkexecDismissed => write!(f, "Polkit dialog dismissed"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJson(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

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

// XXX handle return code from pkexec? Non-zero exit code from hp-vendor-purposes?
fn pkexec<T: serde::de::DeserializeOwned>(cmd: &[&str]) -> Result<T, Error> {
    let output = Command::new("pkexec")
        .args(cmd)
        .output()?;
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Get data colection purposes and opt-in status. Purposes may be `None`
/// if purposes for `locale` are not cached and it is unable to communicate
/// with server. Does not prompt for authentication.
pub fn purposes(locale: &str) -> Result<PurposesOutput, Error> {
    pkexec(&["/usr/libexec/hp-vendor-purposes", locale])
}
