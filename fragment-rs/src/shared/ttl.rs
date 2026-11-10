use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub enum TTL {
    FiveHundredMs,
    OneSecond,
    FiveSeconds,
    TenSeconds,
    ThirtySeconds,
    OneMinute,
    FiveMinutes,
    TenMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    SixHours,
    TwelveHours,
    OneDay,
    SevenDays,
}

impl TTL {
    pub fn as_duration(&self) -> Duration {
        match self {
            Self::FiveHundredMs => Duration::from_millis(500),
            Self::OneSecond => Duration::from_secs(1),
            Self::FiveSeconds => Duration::from_secs(5),
            Self::TenSeconds => Duration::from_secs(10),
            Self::ThirtySeconds => Duration::from_secs(30),
            Self::OneMinute => Duration::from_secs(60),
            Self::FiveMinutes => Duration::from_secs(300),
            Self::TenMinutes => Duration::from_secs(600),
            Self::FifteenMinutes => Duration::from_secs(900),
            Self::ThirtyMinutes => Duration::from_secs(1800),
            Self::OneHour => Duration::from_secs(3600),
            Self::SixHours => Duration::from_secs(21600),
            Self::TwelveHours => Duration::from_secs(43200),
            Self::OneDay => Duration::from_secs(86400),
            Self::SevenDays => Duration::from_secs(604800),
        }
    }
}

impl Default for TTL {
    fn default() -> Self {
        Self::OneMinute
    }
}

pub fn resolve_duration(value: TTL) -> u64 {
    match value {
        TTL::FiveHundredMs => 500,
        TTL::OneSecond => 1000,
        TTL::FiveSeconds => 5000,
        TTL::TenSeconds => 10000,
        TTL::ThirtySeconds => 30000,
        TTL::OneMinute => 60000,
        TTL::FiveMinutes => 300000,
        TTL::TenMinutes => 600000,
        TTL::FifteenMinutes => 900000,
        TTL::ThirtyMinutes => 1800000,
        TTL::OneHour => 3600000,
        TTL::SixHours => 21600000,
        TTL::TwelveHours => 43200000,
        TTL::OneDay => 86400000,
        TTL::SevenDays => 604800000,
    }
}