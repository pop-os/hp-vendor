#![allow(non_snake_case)]

use reqwest::{
    blocking::{Client, Response},
    Method,
};
use std::{collections::HashMap, str::FromStr};

use crate::event::{DeviceOSIds, Events};

const BASE_URL: &str = "API_URL";

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

pub struct Api {
    client: Client,
    ids: DeviceOSIds,
    token_resp: TokenResponse,
}

impl Api {
    pub fn new(ids: DeviceOSIds) -> anyhow::Result<Self> {
        let client = Client::new();
        let resp = client
            .post(format!("{}/data/token", BASE_URL))
            .json(&ids)
            .send()?;

        if resp.status().is_success() {
            Ok(Self {
                client,
                ids,
                token_resp: resp.json()?,
            })
        } else {
            Err(err_from_resp(resp))
        }
    }

    fn url(&self, name: &str) -> anyhow::Result<(Method, String)> {
        let (method, path) = self
            .token_resp
            .paths
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no url for '{}'", name))?;
        let method = reqwest::Method::from_str(&method)?;
        let dID = &self.token_resp.dID;
        let osID = &self.ids.os_install_uuid;
        let path = path.replace("{dID}", dID).replace("{osID}", osID);
        Ok((method, format!("{}{}", BASE_URL, path)))
    }

    fn request<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        body: &T,
    ) -> anyhow::Result<U> {
        let (method, url) = self.url(name)?;
        let resp = self
            .client
            .request(method, url)
            .header("authorizationToken", &self.token_resp.token)
            .json(body)
            .send()?;
        if resp.status().is_success() {
            Ok(resp.json()?)
        } else {
            Err(err_from_resp(resp))
        }
    }

    pub fn upload(&self, events: &Events) -> anyhow::Result<serde_json::Value> {
        self.request("DataUpload", events)
    }
}

fn err_from_resp(resp: Response) -> anyhow::Error {
    let status = resp.status();
    if let Ok(error) = resp.json::<ErrorResponse>() {
        anyhow::anyhow!("{}: {}", status, error.message)
    } else {
        anyhow::anyhow!("{}", status)
    }
}
