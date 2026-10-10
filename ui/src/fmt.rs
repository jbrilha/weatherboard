#![allow(unused_macros)]

macro_rules! debug {
    ($($arg:tt)*) => {{
        #[cfg(target_os = "none")]
        defmt::debug!($($arg)*);
        #[cfg(not(target_os = "none"))]
        log::debug!($($arg)*);
    }};
}

macro_rules! info {
    ($($arg:tt)*) => {{
        #[cfg(target_os = "none")]
        defmt::info!($($arg)*);
        #[cfg(not(target_os = "none"))]
        log::info!($($arg)*);
    }};
}

macro_rules! warn {
    ($($arg:tt)*) => {{
        #[cfg(target_os = "none")]
        defmt::warn!($($arg)*);
        #[cfg(not(target_os = "none"))]
        log::warn!($($arg)*);
    }};
}

macro_rules! error {
    ($($arg:tt)*) => {{
        #[cfg(target_os = "none")]
        defmt::error!($($arg)*);
        #[cfg(not(target_os = "none"))]
        log::error!($($arg)*);
    }};
}
