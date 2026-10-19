use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Lisbon,
    Caparica,
}

impl Location {
    pub const fn coords(self) -> (f32, f32) {
        match self {
            Location::Lisbon => (38.71667, -9.13333),
            Location::Caparica => (38.66667, -9.2),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Location::Lisbon => "Lisbon",
            Location::Caparica => "Caparica",
        }
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Sequence, EnumString, strum::Display,
)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Day {
    pub fn as_str(&self) -> &'static str {
        match self {
            Day::Sunday => "Sunday",
            Day::Monday => "Monday",
            Day::Tuesday => "Tuesday",
            Day::Wednesday => "Wednesday",
            Day::Thursday => "Thursday",
            Day::Friday => "Friday",
            Day::Saturday => "Saturday",
        }
    }

    pub fn as_short_str(&self) -> &'static str {
        match self {
            Day::Sunday => "Sun",
            Day::Monday => "Mon",
            Day::Tuesday => "Tue",
            Day::Wednesday => "Wed",
            Day::Thursday => "Thu",
            Day::Friday => "Fri",
            Day::Saturday => "Sat",
        }
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Sequence, EnumString, strum::Display,
)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    pub fn as_str(&self) -> &'static str {
        match self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        }
    }

    pub fn as_short_str(&self) -> &'static str {
        match self {
            Month::January => "Jan",
            Month::February => "Feb",
            Month::March => "Mar",
            Month::April => "Apr",
            Month::May => "May",
            Month::June => "Jun",
            Month::July => "Jul",
            Month::August => "Aug",
            Month::September => "Sep",
            Month::October => "Oct",
            Month::November => "Nov",
            Month::December => "Dec",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Condition {
    Sunny,
    PartlyCloudy,
    Cloudy,
    Fog,
    Rain,
    HeavyRain,
    Snow,
    Thunderstorm,
    Wind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DayForecast {
    pub condition: Condition,
    pub low_c: i8,
    pub high_c: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WeatherPayload {
    pub location: Location,
    pub week_day: Day,
    pub month: Month,
    pub day: u8,
    pub updated: (u8, u8),

    pub condition: Condition,
    pub temp_c: i8,
    pub feels_like_c: i8,

    pub sunrise: (u8, u8),
    pub sunset: (u8, u8),
    pub wind_kmh: u8,
    pub humidity_pct: u8,
    pub pressure_hpa: u16,
    pub uvi_tenths: u8,
    pub dew_point_c: i8,
    pub rain_pct: u8,

    pub chart_low_c: i8,
    pub chart_high_c: i8,
    pub chart_start_hour: u8,
    pub chart_temps_c: [i8; 12],
    pub chart_pop_pct: [u8; 12],

    pub forecast: [DayForecast; 6],
}

impl WeatherPayload {
    pub fn new(
        location: Location,
        week_day: Day,
        month: Month,
        day: u8,
        updated: (u8, u8),
        condition: Condition,
        temp_c: i8,
        feels_like_c: i8,
        sunrise: (u8, u8),
        sunset: (u8, u8),
        wind_kmh: u8,
        humidity_pct: u8,
        pressure_hpa: u16,
        uvi_tenths: u8,
        dew_point_c: i8,
        rain_pct: u8,
        chart_low_c: i8,
        chart_high_c: i8,
        chart_start_hour: u8,
        chart_temps_c: [i8; 12],
        chart_pop_pct: [u8; 12],
        forecast: [DayForecast; 6],
    ) -> Self {
        Self {
            location,
            week_day,
            month,
            day,
            updated,
            condition,
            temp_c,
            feels_like_c,
            sunrise,
            sunset,
            wind_kmh,
            humidity_pct,
            pressure_hpa,
            uvi_tenths,
            dew_point_c,
            rain_pct,
            chart_low_c,
            chart_high_c,
            chart_start_hour,
            chart_temps_c,
            chart_pop_pct,
            forecast,
        }
    }

    pub fn default() -> Self {
        let forecast = [
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
            DayForecast {
                condition: Condition::Sunny,
                low_c: 10,
                high_c: 30,
            },
        ];
        Self {
            location: Location::Lisbon,
            week_day: Day::Sunday,
            month: Month::October,
            day: 18,
            updated: (12, 0),
            condition: Condition::Sunny,
            temp_c: 25,
            feels_like_c: 26,
            sunrise: (7, 15),
            sunset: (18, 55),
            wind_kmh: 19,
            humidity_pct: 65,
            pressure_hpa: 1234,
            uvi_tenths: 12,
            dew_point_c: 11,
            rain_pct: 0,
            chart_low_c: 18,
            chart_high_c: 34,
            chart_start_hour: (7),
            chart_temps_c: [18, 19, 21, 24, 27, 30, 32, 34, 31, 27, 23, 20],
            chart_pop_pct: [0, 0, 5, 10, 20, 40, 60, 40, 20, 10, 0, 0],
            forecast,
        }
    }
}
