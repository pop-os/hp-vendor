// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, Read},
    path::Path,
    process::{self, Command, ExitStatus, Stdio},
    str,
};

const DEFAULT_ENDPOINT_URL: &str = "https://api.data.hpdevone.com";
const PURPOSES_CMD: &str = "/usr/libexec/hp-vendor-purposes";
const CMD: &str = "/usr/libexec/hp-vendor";
const CONF_PATH: &str = "/etc/hp-vendor.conf";

#[doc(hidden)]
#[derive(Default, serde::Deserialize)]
pub struct HpVendorConf {
    endpoint_url: Option<String>,
    #[serde(default)]
    pub allow_unsupported_hardware: bool,
}

impl HpVendorConf {
    pub fn endpoint_url(&self) -> &str {
        self.endpoint_url.as_deref().unwrap_or(DEFAULT_ENDPOINT_URL)
    }
}

#[doc(hidden)]
pub fn hp_vendor_conf() -> &'static HpVendorConf {
    static CONF: Lazy<HpVendorConf> = Lazy::new(|| {
        let bytes = match fs::read(CONF_PATH) {
            Ok(bytes) => bytes,
            Err(_) => {
                return HpVendorConf::default();
            }
        };
        toml::from_slice(&bytes).unwrap_or_else(|err| {
            eprintln!("Failed to parse `{}`: {}", CONF_PATH, err);
            HpVendorConf::default()
        })
    });
    &CONF
}

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

    pub fn wait(&mut self) -> Result<(), Error> {
        check_pkexec_status(self.child.wait()?)
    }
}

impl io::Read for Download {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }
}

fn check_pkexec_status(status: ExitStatus) -> Result<(), Error> {
    match status.code() {
        Some(0) => Ok(()),
        Some(126) => Err(Error::PkexecDismissed),
        Some(127) => Err(Error::PkexecNoauth),
        // TODO: Collect stderr, or something?
        _ => Err(Error::HpVendorFailed),
    }
}

/// Get data colection purposes and opt-in status. Does not prompt for authentication.
pub fn purposes() -> Result<PurposesOutput, Error> {
    let output = Command::new("pkexec").args(&[PURPOSES_CMD]).output()?;
    check_pkexec_status(output.status)?;
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Sets consent info in db, and enables daemon
pub fn consent(locale: &str, country: &str, purpose_id: &str, version: &str) -> Result<(), Error> {
    let status = Command::new("pkexec")
        .args(&[CMD, "consent", locale, country, purpose_id, version])
        .status()?;
    check_pkexec_status(status)
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
        .spawn()?;
    let mut stdout = child.stdout.take().unwrap();

    let mut length = [0; 8];
    if let Err(err) = stdout.read_exact(&mut length) {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            check_pkexec_status(child.wait()?)?;
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
    let status = Command::new("pkexec").args(&[CMD, "delete"]).status()?;
    check_pkexec_status(status)
}

/// Disable daemon
pub fn disable() -> Result<(), Error> {
    let status = Command::new("pkexec").args(&[CMD, "disable"]).status()?;
    check_pkexec_status(status)
}

pub fn has_hp_vendor() -> bool {
    Path::new(CMD).exists()
}

pub fn supported_hardware() -> Result<(), String> {
    if hp_vendor_conf().allow_unsupported_hardware {
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
