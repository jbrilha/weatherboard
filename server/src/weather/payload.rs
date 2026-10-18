use common::weather::WeatherPayload;

use super::openweathermap::{Forecast1Hour, HourlyForecast};

const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;

pub fn fill_chart(payload: &mut WeatherPayload, hourly: &Forecast1Hour, now: i64) -> bool {
    let picked: Vec<&HourlyForecast> = hourly
        .data
        .iter()
        .skip_while(|h| h.dt + HOUR <= now)
        .step_by(2)
        .take(12)
        .collect();
    let Ok(picked): Result<[&HourlyForecast; 12], _> = picked.try_into() else {
        return false;
    };

    let temps = picked.map(|h| h.temp.round() as i8);
    let local = picked[0].dt + i64::from(hourly.timezone_offset);

    payload.chart_start_hour = (local.rem_euclid(DAY) / HOUR) as u8;
    payload.chart_temps_c = temps;
    payload.chart_pop_pct = picked.map(|h| (h.pop.unwrap_or(0.0) * 100.0).round() as u8);
    payload.chart_low_c = *temps.iter().min().unwrap();
    payload.chart_high_c = *temps.iter().max().unwrap();
    true
}
