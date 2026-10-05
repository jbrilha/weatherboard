use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Ip {
    pub origin: String,
}

pub async fn weather_test() -> anyhow::Result<Ip> {
    let bytes = reqwest::get("https://httpbin.org/ip")
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(serde_json::from_slice(&bytes)?)
}
