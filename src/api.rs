use reqwest::blocking::Client;

use crate::event::Event;

const TOKEN_URL: &str = "API_URL";
const UPLOAD_URL: &str =
    "API_URL";

#[derive(serde::Serialize)]
pub struct TokenRequest {
    pub devicesn: String,
    pub biosuuid: String,
}

impl TokenRequest {
    pub fn send(&self, client: &Client) -> reqwest::Result<reqwest::blocking::Response> {
        client.post(TOKEN_URL).json(self).send()
    }
}

impl Event {
    pub fn send(
        &self,
        client: &Client,
        token: &str,
    ) -> reqwest::Result<reqwest::blocking::Response> {
        client
            .post(UPLOAD_URL)
            .header("authorizationToken", token)
            .json(self)
            .send()
    }
}
