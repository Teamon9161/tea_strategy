mod future_ret;
mod future_ret_spread;
mod tick_future_ret;

pub use future_ret::{calc_future_ret, FutureRetKwargs};
pub use future_ret_spread::{calc_future_ret_with_spread, FutureRetSpreadKwargs};
use serde::{Deserialize, Deserializer};
use tevec::prelude::{tbail, TResult};
pub use tick_future_ret::{calc_tick_future_ret, TickFutureRetKwargs};

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
            _ => tbail!("invalid commision type"),
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
