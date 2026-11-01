#![no_std]

use common::weather::{Condition, Day, DayForecast, Location, Month};
use common::mqtt::Topic;
use common::{Message, ToBoard};

#[unsafe(no_mangle)]
pub extern "C" fn topic_str(topic: Topic) -> *const core::ffi::c_char {
    topic.as_c_str().as_ptr()
}

#[repr(C)]
pub struct Weather {
    pub location: Location,
    pub week_day: Day,
    pub month: Month,
    pub day: u8,
    pub updated: [u8; 2],
    pub condition: Condition,
    pub temp_c: i8,
    pub feels_like_c: i8,
    pub sunrise: [u8; 2],
    pub sunset: [u8; 2],
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_weather(buf: *const u8, len: usize, out: *mut Weather) -> bool {
    let bytes = unsafe { core::slice::from_raw_parts(buf, len) };
    let Ok(ToBoard::Weather(w)) = ToBoard::decode(bytes) else {
        return false;
    };
    unsafe {
        out.write(Weather {
            location: w.location,
            week_day: w.week_day,
            month: w.month,
            day: w.day,
            updated: [w.updated.0, w.updated.1],
            condition: w.condition,
            temp_c: w.temp_c,
            feels_like_c: w.feels_like_c,
            sunrise: [w.sunrise.0, w.sunrise.1],
            sunset: [w.sunset.0, w.sunset.1],
            wind_kmh: w.wind_kmh,
            humidity_pct: w.humidity_pct,
            pressure_hpa: w.pressure_hpa,
            uvi_tenths: w.uvi_tenths,
            dew_point_c: w.dew_point_c,
            rain_pct: w.rain_pct,
            chart_low_c: w.chart_low_c,
            chart_high_c: w.chart_high_c,
            chart_start_hour: w.chart_start_hour,
            chart_temps_c: w.chart_temps_c,
            chart_pop_pct: w.chart_pop_pct,
            forecast: w.forecast,
        });
    }
    true
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
