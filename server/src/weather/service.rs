use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use common::weather::Location;
use tokio::sync::watch;

use crate::weather::{
    WeatherParams,
    openweathermap::{Forecast1Day, Forecast1Hour, OpenCallEndpoint, Units},
};

use super::{WeatherRequest, client::WeatherClient};

const REFRESH_INTERVAL: Duration = Duration::from_mins(15);
const RETRY_INTERVAL: Duration = Duration::from_secs(30);

pub struct WeatherService {
    client: WeatherClient,
    request: WeatherRequest,
    hourly_request: WeatherRequest,
    latest: watch::Sender<Option<Arc<Forecast1Day>>>,
    latest_hourly: watch::Sender<Option<Arc<Forecast1Hour>>>,
    last_error: Mutex<Option<String>>,
}

impl WeatherService {
    pub fn new(
        client: WeatherClient,
        request: WeatherRequest,
        hourly_request: WeatherRequest,
    ) -> Self {
        let (latest, _) = watch::channel(None);
        let (latest_hourly, _) = watch::channel(None);
        Self {
            client,
            request,
            hourly_request,
            latest,
            latest_hourly,
            last_error: Mutex::new(None),
        }
    }

    pub fn default() -> Self {
        let client = WeatherClient::new();
        let request = WeatherRequest::new(
            OpenCallEndpoint::Timeline1Day,
            WeatherParams::at(Location::Lisbon, Units::Metric),
        );
        let hourly_request = WeatherRequest::new(
            OpenCallEndpoint::Timeline1Hour,
            WeatherParams::at(Location::Lisbon, Units::Metric),
        );
        WeatherService::new(client, request, hourly_request)
    }

    pub fn latest(&self) -> Option<Arc<Forecast1Day>> {
        self.latest.borrow().clone()
    }

    pub fn latest_hourly(&self) -> Option<Arc<Forecast1Hour>> {
        self.latest_hourly.borrow().clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let fetched = tokio::try_join!(
            self.client.fetch(&self.request),
            self.client.fetch(&self.hourly_request),
        );
        match fetched {
            Ok((daily, hourly)) => {
                self.latest.send_replace(Some(Arc::new(daily)));
                self.latest_hourly.send_replace(Some(Arc::new(hourly)));
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
