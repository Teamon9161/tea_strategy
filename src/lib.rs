mod auto_boll;
mod boll;

mod martingale;
mod strategy_filter;

pub mod equity;

pub use auto_boll::{auto_boll, AutoBollKwargs};
pub use boll::{boll, BollKwargs};
pub use martingale::{martingale, MartingaleKwargs};
pub use strategy_filter::StrategyFilter;
pub use tevec;
