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
