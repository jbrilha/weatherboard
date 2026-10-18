use std::time::Duration;

use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::weather::openweathermap::{Forecast, ONE_CALL_API};

use super::WeatherRequest;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WeatherClient {
    pub client: reqwest::Client,
    pub api_key: String,
}

impl WeatherClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build HTTP client");
        WeatherClient::new_from(client)
    }

    pub fn new_from(client: reqwest::Client) -> Self {
        let api_key = std::env::var("OPENWEATHER_API_KEY")
            .ok()
            .expect("failed to get API key");
        Self { client, api_key }
    }

    pub async fn fetch<T: DeserializeOwned>(
        &self,
        req: &WeatherRequest,
    ) -> anyhow::Result<Forecast<T>> {
        let url = Url::parse(ONE_CALL_API)?.join(req.endpoint.path())?;

        let response = self
            .client
            .get(url)
            .query(&req.params)
            .query(&[("appid", self.api_key.clone())])
            .send()
            .await
            .map_err(reqwest::Error::without_url)?
            .error_for_status()
            .map_err(reqwest::Error::without_url)?
            .json::<Forecast<T>>()
            .await
            .map_err(reqwest::Error::without_url)?;

        Ok(response)
    }
}
