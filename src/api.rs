use reqwest::blocking::Client;

use crate::event::{self, DeviceOSIds, Events};

const TOKEN_URL: &str = "https://ipngm19tbi.execute-api.us-east-1.amazonaws.com/test/token";
const UPLOAD_URL: &str =
    "https://ipngm19tbi.execute-api.us-east-1.amazonaws.com/test/events/upload";

#[derive(Debug, serde::Serialize)]
pub struct TokenRequest(DeviceOSIds);

#[derive(Debug, serde::Deserialize)]
pub struct TokenResponse {
    pub message: String,
    pub token: String,
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

impl TokenRequest {
    pub fn new(ids: DeviceOSIds) -> Self {
        Self(ids)
    }

    pub fn send(&self, client: &Client) -> reqwest::Result<TokenResponse> {
        client
            .post(TOKEN_URL)
            .header("x-api-key", option_env!("API_KEY").unwrap_or(""))
            .json(self)
            .send()?
            .json()
    }
}

impl<'a> Events<'a> {
    pub fn send(&self, client: &Client, token: &str) -> reqwest::Result<serde_json::Value> {
        client
            .post(UPLOAD_URL)
            .header("x-api-key", option_env!("API_KEY").unwrap_or(""))
            .header("authorizationToken", token)
            .json(self)
            .send()?
            .json()
    }
}
