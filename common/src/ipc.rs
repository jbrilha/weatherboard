use crate::Payload;

pub const HUB_SOCKET: &str = "/tmp/weatherboard.sock";
pub enum ToServer {
    GetWeather,
}
pub enum ToHub {
    Weather(Payload),
}
