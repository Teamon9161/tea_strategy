mod future_ret;
mod future_ret_spread;
mod tick_future_ret;
mod tick_future_ret_full;

pub use future_ret::{calc_future_ret, FutureRetKwargs};
pub use future_ret_spread::{calc_future_ret_with_spread, FutureRetSpreadKwargs};
use serde::{Deserialize, Deserializer};
use tevec::prelude::{tbail, TResult};
pub use tick_future_ret::{calc_tick_future_ret, TickFutureRetKwargs};
pub use tick_future_ret_full::{
    calc_tick_future_ret_full, profit_vec_to_series, OpenPriceMethod, Profit,
    TickFutureRetFullKwargs,
};

#[derive(Clone, Copy)]
pub enum CommissionType {
    Percent,
    Absolute,
}

impl CommissionType {
    #[inline]
    pub fn parse(s: &str) -> TResult<Self> {
        match s.to_lowercase().as_str() {
            "percent" | "pct" => Ok(CommissionType::Percent),
            "fixed" | "absolute" | "fix" => Ok(CommissionType::Absolute),
            _ => tbail!("invalid commission type"),
        }
    }
}

impl<'de> Deserialize<'de> for CommissionType {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        CommissionType::parse(s.as_str()).map_err(serde::de::Error::custom)
    }
}

impl<A: AsRef<str>> From<A> for CommissionType {
    #[inline]
    fn from(s: A) -> Self {
        CommissionType::parse(s.as_ref()).unwrap()
    }
}

#[derive(Clone, Copy)]
pub enum SignalType {
    Percent,
    Absolute,
}

impl SignalType {
    #[inline]
    pub fn parse(s: &str) -> TResult<Self> {
        match s.to_lowercase().as_str() {
            "percent" | "pct" => Ok(SignalType::Percent),
            "fixed" | "absolute" | "fix" => Ok(SignalType::Absolute),
            _ => tbail!("invalid signal type"),
        }
    }
}

impl<'de> Deserialize<'de> for SignalType {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SignalType::parse(s.as_str()).map_err(serde::de::Error::custom)
    }
}
