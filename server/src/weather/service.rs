use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use common::weather::Location;
use serde::de::DeserializeOwned;
use tokio::sync::watch;

use crate::weather::{
    WeatherParams,
    openweathermap::{Forecast, Forecast1Day, Forecast1Hour, OpenCallEndpoint, Units},
};

use super::{WeatherRequest, client::WeatherClient};

const REFRESH_INTERVAL: Duration = Duration::from_mins(15);
const RETRY_INTERVAL: Duration = Duration::from_secs(30);

pub struct Snapshot {
    pub daily: Arc<Forecast1Day>,
    pub hourly: Forecast1Hour,
    pub fetched_at: i64,
}

pub struct WeatherService {
    client: WeatherClient,
    location: Location,
    daily_request: WeatherRequest,
    hourly_request: WeatherRequest,
    latest: watch::Sender<Option<Arc<Snapshot>>>,
    last_error: Mutex<Option<String>>,
}

impl WeatherService {
    pub fn new(client: WeatherClient, location: Location, units: Units) -> Self {
        let (latest, _) = watch::channel(None);
        Self {
            client,
            location,
            daily_request: WeatherRequest::new(
                OpenCallEndpoint::Timeline1Day,
                WeatherParams::at(location, units),
            ),
            hourly_request: WeatherRequest::new(
                OpenCallEndpoint::Timeline1Hour,
                WeatherParams::at(location, units),
            ),
            latest,
            last_error: Mutex::new(None),
        }
    }

    pub fn default() -> Self {
        WeatherService::new(WeatherClient::new(), Location::Lisbon, Units::Metric)
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn latest(&self) -> Option<Arc<Snapshot>> {
        self.latest.borrow().clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    async fn fetch_paged<T: DeserializeOwned>(
        &self,
        request: &WeatherRequest,
    ) -> anyhow::Result<Forecast<T>> {
        let mut forecast: Forecast<T> = self.client.fetch(request).await?;
        if let Some(next) = forecast.next.take() {
            let more: Forecast<T> = self.client.fetch_next(&next).await?;
            forecast.data.extend(more.data);
        }
        Ok(forecast)
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let fetched = tokio::try_join!(
            self.client.fetch(&self.daily_request),
            self.fetch_paged(&self.hourly_request),
        );
        match fetched {
            Ok((daily, hourly)) => {
                self.latest.send_replace(Some(Arc::new(Snapshot {
                    daily: Arc::new(daily),
                    hourly,
                    fetched_at: jiff::Timestamp::now().as_second(),
                })));
                *self.last_error.lock().unwrap() = None;
                tracing::info!("weather refreshed");
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
