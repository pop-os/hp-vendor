#![allow(non_snake_case)]

use reqwest::{
    blocking::{Client, RequestBuilder},
    Method,
};
use std::{collections::HashMap, str::FromStr};

use crate::event::{DeviceOSIds, Events};

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
        // TODO Handle error response from server?
        let token_resp = client
            .post(format!("{}/data/token", BASE_URL))
            .json(&ids)
            .send()?
            .json()?;
        Ok(Self {
            client,
            ids,
            token_resp,
        })
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

    fn request(&self, name: &str) -> anyhow::Result<RequestBuilder> {
        let (method, url) = self.url(name)?;
        Ok(self
            .client
            .request(method, url)
            .header("authorizationToken", &self.token_resp.token))
    }

    pub fn upload(&self, events: &Events) -> anyhow::Result<serde_json::Value> {
        // TODO handle error response
        Ok(self.request("DataUpload")?.json(events).send()?.json()?)
    }
}
