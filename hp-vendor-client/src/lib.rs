// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{fmt, fs, io, process::Command};

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    Io(io::Error),
    PkexecNoauth,
    PkexecDismissed,
    HpVendorFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SerdeJson(err) => write!(f, "{}", err),
            Self::Io(err) => write!(f, "{}", err),
            Self::PkexecNoauth => write!(f, "Polkit authorization failed"),
            Self::PkexecDismissed => write!(f, "Polkit dialog dismissed"),
            Self::HpVendorFailed => write!(f, "Call to hp-vendor failed"),
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
    /// `true` for opted-in, `false` for opt-out, `None` if no opt-in/out has
    /// been set.
    pub opted: Option<bool>,
    /// May be `None` if purposes for given locale are not cached and
    /// hp-vendor is unable to communicate with the server.
    pub purposes: Option<Vec<DataCollectionPurpose>>,
}

fn pkexec<T: serde::de::DeserializeOwned>(cmd: &[&str]) -> Result<T, Error> {
    let output = Command::new("pkexec").args(cmd).output()?;
    match output.status.code() {
        Some(0) => Ok(serde_json::from_slice(&output.stdout)?),
        Some(126) => Err(Error::PkexecDismissed),
        Some(127) => Err(Error::PkexecNoauth),
        // TODO: Collect stderr, or something?
        _ => Err(Error::HpVendorFailed),
    }
}

/// Get data colection purposes and opt-in status. Does not prompt for authentication.
pub fn purposes(locale: &str) -> Result<PurposesOutput, Error> {
    pkexec(&["/usr/libexec/hp-vendor-purposes", locale])
}

pub fn consent(
    locale: &str,
    country: &str,
    purposes: &[DataCollectionPurpose],
) -> Result<(), Error> {
    todo!()
}

pub fn download(file: &mut fs::File, zip: bool) -> Result<(), Error> {
    // Can't use this pkexec function here...
    todo!()
}

// Or document that disable should be called first?
pub fn delete_and_disable() -> Result<(), Error> {
    // pkexec(&["/usr/libexec/hp-vendor", "delete"])
    // Doesn't need json?
    todo!()
}

pub fn disable() -> Result<(), Error> {
    todo!()
}
