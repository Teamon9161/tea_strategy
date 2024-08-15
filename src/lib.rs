mod strategies;
mod trade;

pub mod equity;

pub use strategies::*;
pub use tevec;
#[cfg(feature = "polars")]
pub use trade::trade_vec_to_series;
pub use trade::{signal_to_trades, PriceVec, Trade, TradeSide};
