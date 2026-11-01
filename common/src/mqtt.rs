use core::ffi::CStr;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Topic {
    Weather,
}

impl Topic {
    pub const fn as_c_str(self) -> &'static CStr {
        match self {
            Topic::Weather => c"weatherboard/weather",
        }
    }

    pub fn as_str(self) -> &'static str {
        self.as_c_str().to_str().unwrap()
    }
}
