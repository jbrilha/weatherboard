use reqwest::Url;
use tracing::info;

use super::{ONE_CALL_API, WeatherInfo, WeatherRequest};

pub struct WeatherClient {
    pub client: reqwest::Client,
    pub api_key: String,
}

impl WeatherClient {
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        WeatherClient::new_from(client)
    }

    pub fn new_from(client: reqwest::Client) -> Self {
        let api_key = std::env::var("OPENWEATHER_API_KEY")
            .ok()
            .expect("failed to get API key");
        Self { client, api_key }
    }

    pub async fn fetch(&self, req: &WeatherRequest) -> anyhow::Result<WeatherInfo> {
        let base = Url::parse(ONE_CALL_API)?.join("timeline/1day")?;
        let params = [
            ("lat", req.lat.to_string()),
            ("lon", req.lon.to_string()),
            ("appid", self.api_key.clone()),
        ];
        let url = Url::parse_with_params(base.as_str(), params)?;

        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<WeatherInfo>()
            .await?;

        println!("{:?}", response.lat);

        Ok(response)
    }
}
