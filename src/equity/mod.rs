mod future_ret;
mod future_ret_spread;
mod tick_future_ret;

use serde::Deserialize;

pub use future_ret::{calc_future_ret, FutureRetKwargs};
pub use future_ret_spread::{calc_future_ret_with_spread, FutureRetSpreadKwargs};
pub use tick_future_ret::{calc_tick_future_ret, TickFutureRetKwargs};

#[derive(Deserialize, Clone, Copy)]
pub enum CommisionType {
    Percent,
    Absolute,
}

impl<A: AsRef<str>> From<A> for CommisionType {
    #[inline]
    fn from(s: A) -> Self {
        match s.as_ref().to_lowercase().as_ref() {
            "percent" => CommisionType::Percent,
            "absolute" => CommisionType::Absolute,
            _ => unreachable!(),
        }
    }
}
