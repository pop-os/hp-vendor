use reqwest::blocking::Client;

use crate::event::Event;

const TOKEN_URL: &str = "https://aezycm9xhe.execute-api.us-east-1.amazonaws.com/test/token";
const UPLOAD_URL: &str =
    "https://aezycm9xhe.execute-api.us-east-1.amazonaws.com/test/events/v1/upload";

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
