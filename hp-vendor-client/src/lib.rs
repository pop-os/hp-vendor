// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, Read, Write},
    path::Path,
    process::{self, Command, ExitStatus, Stdio},
    str,
};

#[doc(hidden)]
pub mod conf;
mod error;
pub use error::*;

const PURPOSES_CMD: &str = "/usr/libexec/hp-vendor-purposes";
const CMD: &str = "/usr/libexec/hp-vendor";

pub fn static_purposes() -> HashMap<String, DataCollectionPurpose> {
    serde_json::from_slice(include_bytes!("../purposes.json")).unwrap()
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DataCollectionPurpose {
    pub purpose_id: String,
    pub version: String,
    pub min_version: String,
    pub statement: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DataCollectionConsent {
    pub country: String,
    pub locale: String,
    pub purpose_id: String,
    pub version: String,
    pub sent: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PurposesOutput {
    /// Purpose opted in to, if any
    pub consent: Option<DataCollectionConsent>,
    /// Purpose, by language. Treat `en` as default.
    pub purposes: HashMap<String, DataCollectionPurpose>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadFormat {
    Json,
    Zip,
    GZip,
}

impl fmt::Display for DownloadFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Zip => write!(f, "zip"),
            Self::GZip => write!(f, "gzip"),
        }
    }
}

impl str::FromStr for DownloadFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "json" => Ok(Self::Json),
            "zip" => Ok(Self::Zip),
            "gzip" => Ok(Self::GZip),
            _ => Err(()),
        }
    }
}

pub struct Download {
    child: process::Child,
    stdout: process::ChildStdout,
    length: u64,
}

impl Download {
    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn wait(self) -> Result<(), Error> {
        drop(self.stdout);
        let output = self.child.wait_with_output()?;
        check_pkexec_status(output.status, output.stderr)
    }
}

impl io::Read for Download {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }
}

fn error_from_stderr(mut stderr: &[u8]) -> Option<(&[u8], ErrorJson)> {
    if stderr.last() == Some(&b'\n') {
        stderr = &stderr[..stderr.len() - 1];
    }
    let idx = stderr.iter().rposition(|x| *x == b'\n').unwrap_or(0);
    let res = serde_json::from_slice(&stderr[idx..]).ok()?;
    Some((&stderr[..=idx], res))
}

fn check_pkexec_status(status: ExitStatus, stderr: Vec<u8>) -> Result<(), Error> {
    let mut output = stderr.as_slice();
    let res = match status.code() {
        Some(0) => Ok(()),
        Some(2) => {
            // Structured error from hp-vendor
            if let Some((start, err)) = error_from_stderr(&stderr) {
                output = start;
                match err {
                    ErrorJson::Api(err) => Err(Error::Api(err)),
                    ErrorJson::Other(message) => Err(Error::HpVendorFailed(Some(message))),
                }
            } else {
                Err(Error::HpVendorFailed(None))
            }
        }
        Some(126) => Err(Error::PkexecDismissed),
        Some(127) => Err(Error::PkexecNoauth),
        _ => Err(Error::HpVendorFailed(None)),
    };

    let mut stderr = io::stderr();
    let _ = stderr.write_all(output);
    let _ = stderr.flush();

    res
}

/// Get data colection purposes and opt-in status. Does not prompt for authentication.
pub fn purposes(fetch: bool) -> Result<PurposesOutput, Error> {
    let mut cmd = Command::new("pkexec");
    cmd.arg(PURPOSES_CMD);
    if !fetch {
        cmd.arg("--no-fetch");
    }
    let output = cmd.output()?;
    check_pkexec_status(output.status, Vec::new())?;
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Sets consent info in db, and enables daemon
pub fn consent(locale: &str, country: &str, purpose_id: &str, version: &str) -> Result<(), Error> {
    let output = Command::new("pkexec")
        .args(&[CMD, "consent", locale, country, purpose_id, version])
        .stderr(Stdio::piped())
        .output()?;
    check_pkexec_status(output.status, output.stderr)
}

pub fn download(format: DownloadFormat) -> Result<Download, Error> {
    let mut child = Command::new("pkexec")
        .args(&[
            CMD,
            "download",
            &format.to_string(),
            "--binary-content-length",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child.stdout.take().unwrap();

    let mut length = [0; 8];
    if let Err(err) = stdout.read_exact(&mut length) {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            let output = child.wait_with_output()?;
            check_pkexec_status(output.status, output.stderr)?;
        }
        return Err(Error::Io(err));
    }
    let length = u64::from_le_bytes(length);

    Ok(Download {
        child,
        stdout,
        length,
    })
}

// Or document that disable should be called first?
pub fn delete_and_disable() -> Result<(), Error> {
    let output = Command::new("pkexec")
        .args(&[CMD, "delete"])
        .stderr(Stdio::piped())
        .output()?;
    check_pkexec_status(output.status, output.stderr)
}

/// Disable daemon
pub fn disable() -> Result<(), Error> {
    let output = Command::new("pkexec")
        .args(&[CMD, "disable"])
        .stderr(Stdio::piped())
        .output()?;
    check_pkexec_status(output.status, output.stderr)
}

pub fn has_hp_vendor() -> bool {
    Path::new(CMD).exists()
}

pub fn supported_hardware() -> Result<(), String> {
    if conf::hp_vendor_conf().allow_unsupported_hardware {
        eprintln!("Skipping `supported_hardware` check due to config setting.");
        return Ok(());
    }
    let board_vendor = fs::read_to_string("/sys/class/dmi/id/board_vendor")
        .map_err(|_| "`board_vendor` not defined")?;
    let board_name = fs::read_to_string("/sys/class/dmi/id/board_name")
        .map_err(|_| "`board_name` not defined")?;
    if (board_vendor.trim(), board_name.trim()) != ("HP", "8A78") {
        Err(format!("`{} {}` unrecognized", board_vendor, board_name))
    } else {
        Ok(())
    }
}
