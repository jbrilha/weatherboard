use std::{sync::Arc, time::Duration};

use tokio::sync::watch;

use crate::weather::{
    WeatherParams,
    locations::Location,
    openweathermap::{Forecast, OpenCallEndpoint, Units},
};

use super::{WeatherRequest, client::WeatherClient};

const REFRESH_INTERVAL: Duration = Duration::from_mins(15);

pub struct WeatherService {
    client: WeatherClient,
    request: WeatherRequest,
    latest: watch::Sender<Option<Arc<Forecast>>>,
}

impl WeatherService {
    pub fn new(client: WeatherClient, request: WeatherRequest) -> Self {
        let (latest, _) = watch::channel(None);
        Self {
            client,
            request,
            latest,
        }
    }

    pub fn default() -> Self {
        let client = WeatherClient::new();
        let request = WeatherRequest::new(
            OpenCallEndpoint::Timeline1Day,
            WeatherParams::at(Location::Lisbon, Units::Metric),
        );
        WeatherService::new(client, request)
    }

    pub fn latest(&self) -> Option<Arc<Forecast>> {
        self.latest.borrow().clone()
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let info = self.client.fetch(&self.request).await?;
        self.latest.send_replace(Some(Arc::new(info)));
        Ok(())
    }

    pub async fn run(self: Arc<Self>) {
        let mut interval = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(err) = self.refresh().await {
                tracing::warn!("weather refresh failed: {err}");
            }
        }
    }
}
