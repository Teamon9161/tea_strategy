mod future_ret;
mod future_ret_spread;
mod tick_future_ret;

pub use future_ret::{calc_future_ret, FutureRetKwargs};
pub use future_ret_spread::{calc_future_ret_with_spread, FutureRetSpreadKwargs};
use serde::{Deserialize, Deserializer};
use tevec::prelude::{tbail, TResult};
pub use tick_future_ret::{calc_tick_future_ret, TickFutureRetKwargs};

#[derive(Clone, Copy)]
pub enum CommisionType {
    Percent,
    Absolute,
}

impl CommisionType {
    #[inline]
    pub fn parse(s: &str) -> TResult<Self> {
        match s.to_lowercase().as_str() {
            "percent" | "pct" => Ok(CommisionType::Percent),
            "fixed" | "absolute" | "fix" => Ok(CommisionType::Absolute),
            _ => tbail!("invalid commision type"),
        }
    }
}

impl<'de> Deserialize<'de> for CommisionType {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        CommisionType::parse(s.as_str()).map_err(serde::de::Error::custom)
    }
}

impl<A: AsRef<str>> From<A> for CommisionType {
    #[inline]
    fn from(s: A) -> Self {
        CommisionType::parse(s.as_ref()).unwrap()
    }
}
