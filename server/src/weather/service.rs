use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use common::weather::{Location, WeatherPayload};
use serde::de::DeserializeOwned;
use tokio::sync::watch;
use tracing::warn;

use crate::weather::{
    WeatherParams,
    openweathermap::{Forecast, Forecast1Day, Forecast1Hour, OpenCallEndpoint, Units},
    payload::{fill_chart, fill_daily},
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
        )
        .and_then(|(daily, hourly)| {
            anyhow::ensure!(
                daily.data.len() >= 7,
                "daily timeline too short: {} entries",
                daily.data.len()
            );
            anyhow::ensure!(
                hourly.data.len() >= 7,
                "hourly timeline too short: {} entries",
                hourly.data.len()
            );
            Ok((daily, hourly))
        });
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

    pub fn payload(&self) -> Option<WeatherPayload> {
        let snapshot = self.latest()?;
        let (daily, hourly) = (&snapshot.daily, &snapshot.hourly);
        let now = jiff::Timestamp::now().as_second();
        let mut payload = WeatherPayload::default();
        if !fill_daily(
            &mut payload,
            daily,
            self.location(),
            now,
            snapshot.fetched_at,
        ) {
            warn!(
                "not enough daily data: {} entries, first dt {:?}, now {now}",
                daily.data.len(),
                daily.data.first().map(|d| d.dt),
            );
            return None;
        }
        if !fill_chart(&mut payload, hourly, now) {
            warn!(
                "not enough hourly data for the chart: {} entries, first dt {:?}, last dt {:?}, now {now}",
                hourly.data.len(),
                hourly.data.first().map(|h| h.dt),
                hourly.data.last().map(|h| h.dt),
            );
            return None;
        }
        Some(payload)
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

    pub fn subscribe(&self) -> watch::Receiver<Option<Arc<Snapshot>>> {
        self.latest.subscribe()
    }
}
