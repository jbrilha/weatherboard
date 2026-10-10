use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::watch;

use crate::weather::{
    WeatherParams,
    locations::Location,
    openweathermap::{Forecast, OpenCallEndpoint, Units},
};

use super::{WeatherRequest, client::WeatherClient};

const REFRESH_INTERVAL: Duration = Duration::from_mins(15);
const RETRY_INTERVAL: Duration = Duration::from_secs(30);

pub struct WeatherService {
    client: WeatherClient,
    request: WeatherRequest,
    latest: watch::Sender<Option<Arc<Forecast>>>,
    last_error: Mutex<Option<String>>,
}

impl WeatherService {
    pub fn new(client: WeatherClient, request: WeatherRequest) -> Self {
        let (latest, _) = watch::channel(None);
        Self {
            client,
            request,
            latest,
            last_error: Mutex::new(None),
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

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        match self.client.fetch(&self.request).await {
            Ok(info) => {
                self.latest.send_replace(Some(Arc::new(info)));
                *self.last_error.lock().unwrap() = None;
                Ok(())
            }
            Err(err) => {
                *self.last_error.lock().unwrap() = Some(format!("{err:#}"));
                Err(err)
            }
        }
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            let delay = match self.refresh().await {
                Ok(()) => REFRESH_INTERVAL,
                Err(err) => {
                    tracing::warn!("weather refresh failed: {err:#}");
                    RETRY_INTERVAL
                }
            };
            tokio::time::sleep(delay).await;
        }
    }
}
