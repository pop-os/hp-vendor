#![allow(non_snake_case)]

use base64::read::DecoderReader;
use reqwest::{
    blocking::{Client, Response},
    Method, StatusCode,
};
use std::{cell::RefCell, collections::HashMap, io::Read, str::FromStr};

use crate::event::{self, DeviceOSIds, Events};

const BASE_URL: &str = "https://ipngm19tbi.execute-api.us-east-1.amazonaws.com/test";

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    #[allow(dead_code)]
    detail: String,
    token: String,
    dID: String,
    paths: HashMap<String, (String, String)>,
}

#[derive(Debug, serde::Deserialize)]
struct ErrorResponse {
    message: String,
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
pub struct Purpose {
    locale: String,
    #[serde(rename = "minVersion")]
    min_version: String,
    statement: String,
    version: String,
}

pub struct Api {
    client: Client,
    ids: DeviceOSIds,
    token_resp: RefCell<TokenResponse>,
}

fn authenticate(client: &Client, ids: &DeviceOSIds) -> anyhow::Result<TokenResponse> {
    let resp = client
        .post(format!("{}/data/token", BASE_URL))
        .json(&ids)
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

    // XXX WIP
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
        Ok((method, format!("{}{}", BASE_URL, path)))
    }

    fn request<T: serde::Serialize>(
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
            if let Some(body) = body {
                req = req.json(body);
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

    pub fn upload(&self, events: &Events) -> anyhow::Result<serde_json::Value> {
        Ok(self.request("DataUpload", &[], Some(events))?.json()?)
    }

    pub fn download(&self, zip: bool) -> anyhow::Result<Vec<u8>> {
        let format = if zip { "ZIP" } else { "JSON" };
        let mut res = self.request("DataDownload", &[("fileFormat", format)], None::<&()>)?;
        let mut bytes = Vec::new();
        if zip {
            DecoderReader::new(&mut res, base64::STANDARD).read_to_end(&mut bytes)?;
        } else {
            res.read_to_end(&mut bytes)?;
        }
        Ok(bytes)
    }

    pub fn delete(&self) -> anyhow::Result<()> {
        self.request("DataDelete", &[], None::<&()>)?;
        Ok(())
    }

    pub fn purposes(&self, locale: &str) -> anyhow::Result<Vec<Purpose>> {
        Ok(self
            .request(
                "DataCollectionPurposes",
                &[("locale", locale), ("latest", "true")],
                None::<&()>,
            )?
            .json()?)
    }

    // XXX WIP
    pub fn consent(&self, locale: &str, version: &str) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .request(
                "DataCollectionConsent",
                &[("optIn", "true"), ("locale", locale), ("version", version)],
                Some(&self.ids),
            )?
            .json()?)
    }

    pub fn exists(&self) -> anyhow::Result<bool> {
        Ok(self
            .request("DataExists", &[], None::<&()>)?
            .json::<ExistsResponse>()?
            .has_data)
    }

    // XXX WIP
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
                None::<&()>,
            )?
            .json()?)
    }
}

fn err_from_resp(resp: Response) -> anyhow::Result<Response> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        Err(if let Ok(error) = resp.json::<ErrorResponse>() {
            anyhow::anyhow!("{}: {}", status, error.message)
        } else {
            anyhow::anyhow!("{}", status)
        })
    }
}
