use core::fmt::Write;

use heapless::String;

pub struct Chart {
    pub hour_txt: [String<4>; 12],
}

impl Chart {
    pub fn new(hours: [u8; 12]) -> Self {
        Self {
            hour_txt: hours.map(hour_label),
        }
    }
}

fn hour_label(hour: u8) -> String<4> {
    let suffix = if hour % 24 < 12 { "AM" } else { "PM" };
    let h12 = match hour % 12 {
        0 => 12,
        h => h,
    };

    let mut txt = String::new();
    write!(txt, "{h12}{suffix}").unwrap();
    txt
}
