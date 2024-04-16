#![feature(return_position_impl_trait_in_trait)]
mod boll;

mod strategy_filter;

pub mod equity;
pub use boll::{boll, BollKwargs};
pub use strategy_filter::StrategyFilter;
