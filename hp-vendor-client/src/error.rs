// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{fmt, io};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiError {
    pub endpoint: String,
    pub code: u16,
    pub canonical_reason: Option<String>,
    pub message: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error = if let Some(reason) = &self.canonical_reason {
            format!("{} {}", self.code, reason)
        } else {
            format!("{}", self.code)
        };
        if let Some(message) = &self.message {
            write!(
                f,
                "'{}' from API endpoint '{}': {}",
                error, self.endpoint, message
            )
        } else {
            write!(f, "'{}' from API endpoint '{}'", error, self.endpoint)
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    Io(io::Error),
    PkexecNoauth,
    PkexecDismissed,
    HpVendorFailed(Option<String>),
    Api(ApiError),
    Reqwest(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SerdeJson(err) => write!(f, "{}", err),
            Self::Io(err) => write!(f, "{}", err),
            Self::PkexecNoauth => write!(f, "Polkit authorization failed"),
            Self::PkexecDismissed => write!(f, "Polkit dialog dismissed"),
            Self::HpVendorFailed(None) => write!(f, "Call to hp-vendor failed"),
            Self::HpVendorFailed(Some(message)) => {
                write!(f, "Call to hp-vendor failed: {}", message)
            }
            Self::Api(err) => write!(f, "{}", err),
            Self::Reqwest(err) => write!(f, "{}", err),
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

#[doc(hidden)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum ErrorJson {
    Api(ApiError),
    Other(String),
    Reqwest(String),
}
