use core::fmt;

use enum_iterator::Sequence;

#[derive(Copy, Clone, Debug, PartialEq, Sequence)]
#[cfg_attr(target_os = "none", derive(defmt::Format))]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Day::Sunday => write!(f, "Sunday"),
            Day::Monday => write!(f, "Monday"),
            Day::Tuesday => write!(f, "Tuesday"),
            Day::Wednesday => write!(f, "Wednesday"),
            Day::Thursday => write!(f, "Thursday"),
            Day::Friday => write!(f, "Friday"),
            Day::Saturday => write!(f, "Saturday"),
        }
    }
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
            Day::Tuesday => "Tues",
            Day::Wednesday => "Wed",
            Day::Thursday => "Thu",
            Day::Friday => "Fri",
            Day::Saturday => "Sat",
        }
    }
}
