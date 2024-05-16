mod future_ret;
mod future_ret_spread;

use serde::Deserialize;

pub use future_ret::{calc_future_ret, FutureRetKwargs};
pub use future_ret_spread::{calc_future_ret_with_spread, FutureRetSpreadKwargs};

#[derive(Deserialize, Clone, Copy)]
pub enum CommisionType {
    Percent,
    Absolute,
}

impl From<String> for CommisionType {
    fn from(s: String) -> Self {
        match s.as_str().to_lowercase().as_ref() {
            "percent" => CommisionType::Percent,
            "absolute" => CommisionType::Absolute,
            _ => unreachable!(),
        }
    }
}

impl<'a> From<&'a str> for CommisionType {
    fn from(s: &'a str) -> Self {
        match s.to_lowercase().as_ref() {
            "percent" => CommisionType::Percent,
            "absolute" => CommisionType::Absolute,
            _ => unreachable!(),
        }
    }
}
