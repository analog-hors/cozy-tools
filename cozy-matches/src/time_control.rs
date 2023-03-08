use std::str::FromStr;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct TimeControl {
    pub time: Duration,
    pub increment: Duration
}

#[derive(Debug, Error, Clone, Copy)]
#[error("invalid time control")]
pub struct InvalidTimeControl;

impl FromStr for TimeControl {
    type Err = InvalidTimeControl;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (time, increment) = s.split_once('+').ok_or(InvalidTimeControl)?;
        let time = parse_duration(time).ok_or(InvalidTimeControl)?;
        let increment = parse_duration(increment).ok_or(InvalidTimeControl)?;
        Ok(TimeControl { time, increment })
    }
}

fn parse_duration(s: &str) -> Option<Duration> {
    fn secs(s: &str) -> Option<Duration> {
        let secs: f64 = s.parse().ok()?;
        if secs.is_sign_negative() || !secs.is_finite() || secs >= u64::MAX as f64 {
            return None;
        }
        Some(Duration::from_secs_f64(secs))
    }

    if let Some(s) = s.strip_suffix("ms") {
        return Some(secs(s)? / 1000);
    }
    if let Some(s) = s.strip_suffix("s") {
        return Some(secs(s)?);
    }
    if let Some(s) = s.strip_suffix("m") {
        return Some(secs(s)? * 60);
    }
    if let Some(s) = s.strip_suffix("h") {
        return Some(secs(s)? * 60 * 60);
    }
    secs(s)
}
