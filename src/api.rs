// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

#![allow(non_snake_case)]

use base64::read::DecoderReader;
use reqwest::{
    blocking::{Client, Response},
    header, Method, StatusCode,
};
use serde_json::Value;
use std::{cell::RefCell, collections::HashMap, error::Error, fmt, io::Read, str::FromStr};

use crate::{
    event::{self, DeviceOSIds, Events},
    util,
};

pub use hp_vendor_client::DownloadFormat;

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    #[allow(dead_code)]
    detail: String,
    token: String,
    dID: String,
    paths: HashMap<String, (String, String)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct EventsResponseDetail {
    pub loc: (String, String, u32, String, String),
    pub msg: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct EventsResponse {
    pub detail: Vec<EventsResponseDetail>,
}

#[derive(Debug, serde::Deserialize)]
struct ExistsResponse {
    has_data: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct PurposeVerbiage {
    pub locale: String,
    #[serde(rename = "minVersion")]
    pub min_version: String,
    pub statement: String,
    pub version: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Purpose {
    pub organization: String,
    pub processingBasis: String,
    #[serde(rename = "purposeId")]
    pub purpose_id: String,
    pub requiredIdentifiers: String,
    pub verbiage: PurposeVerbiage,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConsentResponse {
    pub acknowledgement: bool,
    pub consent_action: String,
}

#[derive(Debug)]
pub struct PayloadSizeError;

impl fmt::Display for PayloadSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Payload too large")
    }
}

impl Error for PayloadSizeError {}

pub struct Api {
    client: Client,
    ids: DeviceOSIds,
    token_resp: RefCell<TokenResponse>,
}

fn authenticate(client: &Client, ids: &DeviceOSIds) -> anyhow::Result<TokenResponse> {
    let resp = client
        .post(format!(
            "{}/data/token",
            util::hp_vendor_conf().endpoint_url()
        ))
        .json(&event::DeviceIds::from(ids))
        .send()?;
    Ok(err_from_resp(resp)?.json()?)
}

impl Api {
    pub fn new(ids: DeviceOSIds) -> anyhow::Result<Self> {
        let client = Client::new();
        let resp = authenticate(&client, &ids)?;
        Ok(Self {
            client,
            ids,
            token_resp: RefCell::new(resp),
        })
    }

    #[allow(dead_code)]
    fn reauthenticate(&self) -> anyhow::Result<()> {
        let resp = authenticate(&self.client, &self.ids)?;
        *self.token_resp.borrow_mut() = resp;
        Ok(())
    }

    fn url(&self, name: &str) -> anyhow::Result<(Method, String)> {
        let token_resp = self.token_resp.borrow();
        let (method, path) = token_resp
            .paths
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no url for '{}'", name))?;
        let method = reqwest::Method::from_str(&method)?;
        let dID = &token_resp.dID;
        let osID = &self.ids.os_install_uuid;
        let path = path.replace("{dID}", dID).replace("{osID}", osID);
        Ok((
            method,
            format!("{}{}", util::hp_vendor_conf().endpoint_url(), path),
        ))
    }

    fn request_inner<T: serde::Serialize>(
        &self,
        name: &str,
        query: &[(&str, &str)],
        body: Option<&T>,
    ) -> anyhow::Result<Response> {
        let mut reauthenticated = false;
        loop {
            let (method, url) = self.url(name)?;
            let mut req = self
                .client
                .request(method, url)
                .header("authorizationToken", &self.token_resp.borrow().token)
                .query(query);
            if let Some(body) = &body {
                // Like `RequestBuilder::json`, use `serde_json::to_vec` and set header
                let body = serde_json::to_vec(body)?;
                if body.len() >= 8000 {
                    return Err(PayloadSizeError.into());
                }
                req = req.header(header::CONTENT_TYPE, "application/json");
                req = req.body(body);
            }
            let resp = req.send()?;
            if !reauthenticated
                && resp.status() == StatusCode::FORBIDDEN
                && resp.headers().get("x-amzn-errortype").map(|x| x.as_bytes())
                    == Some(b"AccessDeniedException")
            {
                self.reauthenticate()?;
                reauthenticated = true;
            } else {
                return err_from_resp(resp);
            }
        }
    }

    fn request(&self, name: &str, query: &[(&str, &str)]) -> anyhow::Result<Response> {
        self.request_inner(name, query, None::<&()>)
    }

    fn request_json<T: serde::Serialize>(
        &self,
        name: &str,
        query: &[(&str, &str)],
        json: &T,
    ) -> anyhow::Result<Response> {
        self.request_inner(name, query, Some(json))
    }

    pub fn upload(&self, events: &Events) -> anyhow::Result<serde_json::Value> {
        Ok(self.request_json("DataUpload", &[], events)?.json()?)
    }

    pub fn download(&self, format: DownloadFormat) -> anyhow::Result<Box<dyn Read>> {
        let res = self.request(
            "DataDownload",
            &[("fileFormat", &format.to_string().to_ascii_uppercase())],
        )?;
        if format != DownloadFormat::Json {
            Ok(Box::new(DecoderReader::from(
                res,
                &base64::engine::DEFAULT_ENGINE,
            )))
        } else {
            Ok(Box::new(res))
        }
    }

    pub fn delete(&self) -> anyhow::Result<()> {
        self.request("DataDelete", &[])?;
        Ok(())
    }

    pub fn purposes(
        &self,
        locale: Option<&str>,
    ) -> anyhow::Result<HashMap<String, event::DataCollectionPurpose>> {
        let params = match locale {
            Some(locale) => vec![("locale", locale), ("latest", "true")],
            None => vec![("latest", "true")],
        };
        let purposes: Vec<Purpose> = self.request("DataCollectionPurposes", &params)?.json()?;
        Ok(purposes
            .into_iter()
            .map(|purpose| {
                (
                    purpose.verbiage.locale.clone(),
                    event::DataCollectionPurpose {
                        purpose_id: purpose.purpose_id,
                        version: purpose.verbiage.version,
                        min_version: purpose.verbiage.min_version,
                        statement: purpose.verbiage.statement,
                    },
                )
            })
            .collect())
    }

    pub fn consent(
        &self,
        locale: &str,
        country: &str,
        purpose_id: &str,
        version: &str,
    ) -> anyhow::Result<ConsentResponse> {
        Ok(self
            .request_json(
                "DataCollectionConsent",
                &[
                    ("optIn", "true"),
                    ("locale", locale),
                    ("country", country),
                    ("purposeId", purpose_id),
                    ("version", version),
                ],
                &self.ids,
            )?
            .json()?)
    }

    pub fn exists(&self) -> anyhow::Result<bool> {
        Ok(self
            .request("DataExists", &[])?
            .json::<ExistsResponse>()?
            .has_data)
    }

    pub fn config(&self) -> anyhow::Result<crate::config::Config> {
        let data_provider = event::data_provider();
        Ok(self
            .request(
                "DataConfig",
                &[
                    ("appName", &data_provider.app_name),
                    ("appVersion", &data_provider.app_version),
                    ("osName", &data_provider.os_name),
                    ("osVersion", &data_provider.os_version),
                ],
            )?
            .json()?)
    }
}

fn err_from_resp(resp: Response) -> anyhow::Result<Response> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        if let Ok(Value::Object(obj)) = resp.json::<Value>() {
            if let Some(Value::String(message)) = obj.get("message") {
                return Err(anyhow::anyhow!("{}: {}", status, message));
            } else if let Some(Value::String(detail)) = obj.get("detail") {
                return Err(anyhow::anyhow!("{}: {}", status, detail));
            }
        }
        Err(anyhow::anyhow!("{}", status))
    }
}
