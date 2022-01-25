use reqwest::blocking::Client;

use crate::event::Events;

const TOKEN_URL: &str = "API_URL";
const UPLOAD_URL: &str =
    "API_URL";

#[derive(Debug, serde::Serialize)]
pub struct TokenRequest {
    pub devicesn: String,
    pub biosuuid: String,
}

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
    detail: Vec<EventsResponseDetail>,
}

impl TokenRequest {
    pub fn send(&self, client: &Client) -> reqwest::Result<TokenResponse> {
        client
            .post(TOKEN_URL)
            .header("x-api-key", option_env!("API_KEY").unwrap_or(""))
            .json(self)
            .send()?
            .json()
    }
}

impl Events {
    pub fn send(&self, client: &Client, token: &str) -> reqwest::Result<EventsResponse> {
        client
            .post(UPLOAD_URL)
            .header("x-api-key", option_env!("API_KEY").unwrap_or(""))
            .header("authorizationToken", token)
            .json(self)
            .send()?
            .json()
    }
}
