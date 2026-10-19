use common::weather::{Condition, Day, DayForecast, Location, Month, WeatherPayload};
use enum_iterator::all;

use super::openweathermap::{
    DailyForecast, Forecast1Day, Forecast1Hour, HourlyForecast, Weather, WeatherMain,
};

const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;

fn local_day(t: i64, offset: i64) -> i64 {
    (t + offset).div_euclid(DAY)
}

fn hours_minutes(t: i32, offset: i64) -> (u8, u8) {
    let secs = (i64::from(t) + offset).rem_euclid(DAY);
    ((secs / HOUR) as u8, (secs % HOUR / 60) as u8)
}

fn condition(weather: &[Weather]) -> Condition {
    let Some(w) = weather.first() else {
        return Condition::Cloudy;
    };
    match w.main {
        WeatherMain::Clear => Condition::Sunny,
        WeatherMain::Clouds if w.id <= 802 => Condition::PartlyCloudy,
        WeatherMain::Clouds => Condition::Cloudy,
        WeatherMain::Rain if matches!(w.id, 502..=504 | 522 | 531) => Condition::HeavyRain,
        WeatherMain::Rain | WeatherMain::Drizzle => Condition::Rain,
        WeatherMain::Snow => Condition::Snow,
        WeatherMain::Thunderstorm => Condition::Thunderstorm,
        WeatherMain::Squall | WeatherMain::Tornado => Condition::Wind,
        WeatherMain::Ash
        | WeatherMain::Dust
        | WeatherMain::Fog
        | WeatherMain::Haze
        | WeatherMain::Mist
        | WeatherMain::Sand
        | WeatherMain::Smoke => Condition::Fog,
    }
}

pub fn fill_daily(
    payload: &mut WeatherPayload,
    daily: &Forecast1Day,
    location: Location,
    now: i64,
) -> bool {
    let offset = i64::from(daily.timezone_offset);
    let today = local_day(now, offset);
    let days: Vec<&DailyForecast> = daily
        .data
        .iter()
        .skip_while(|d| local_day(i64::from(d.dt), offset) < today)
        .take(7)
        .collect();
    let Ok([t, forecast @ ..]): Result<[&DailyForecast; 7], _> = days.try_into() else {
        return false;
    };
    let Ok(local_now) = jiff::Timestamp::from_second(now + offset) else {
        return false;
    };
    let date = local_now.to_zoned(jiff::tz::TimeZone::UTC).date();

    payload.location = location;
    payload.week_day = all::<Day>()
        .nth(date.weekday().to_sunday_zero_offset() as usize)
        .unwrap();
    payload.month = all::<Month>().nth(date.month() as usize - 1).unwrap();
    payload.day = date.day() as u8;

    payload.condition = condition(&t.weather);
    payload.temp_c = t.temp.day.round() as i8;
    payload.feels_like_c = t.feels_like.day.round() as i8;

    payload.sunrise = hours_minutes(t.sunrise, offset);
    payload.sunset = hours_minutes(t.sunset, offset);
    payload.wind_kmh = (t.wind_speed * 3.6).round() as u8;
    payload.humidity_pct = t.humidity.clamp(0, 100) as u8;
    payload.pressure_hpa = t.pressure.round() as u16;
    payload.uvi_tenths = (t.uvi.unwrap_or(0.0) * 10.0).round() as u8;
    payload.dew_point_c = t.dew_point.round() as i8;
    payload.rain_pct = (t.pop.unwrap_or(0.0) * 100.0).round() as u8;

    payload.forecast = forecast.map(|d| DayForecast {
        condition: condition(&d.weather),
        low_c: d.temp.min.round() as i8,
        high_c: d.temp.max.round() as i8,
    });
    true
}

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
