// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use once_cell::sync::Lazy;
use std::fs;

const DEFAULT_ENDPOINT_URL: &str = "https://api.data.hpdevone.com";
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
