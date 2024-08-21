use itertools::izip;
#[cfg(feature = "polars")]
use tevec::export::polars::prelude::{BooleanChunked, DataFrame};
use tevec::prelude::*;

#[derive(Clone)]
pub struct StrategyFilter<T: Vec1View<Option<bool>>> {
    pub long_open: T,
    pub long_stop: T,
    pub short_open: T,
    pub short_stop: T,
}

pub type FilterElement = (Option<bool>, Option<bool>, Option<bool>, Option<bool>);

impl<T: Vec1View<Option<bool>>> StrategyFilter<T> {
    pub fn titer(&self) -> TrustIter<impl Iterator<Item = FilterElement> + '_> {
        let iter = izip!(
            self.long_open.titer(),
            self.long_stop.titer(),
            self.short_open.titer(),
            self.short_stop.titer()
        );
        TrustIter::new(iter, self.long_open.len())
    }
}

#[cfg(feature = "polars")]
impl From<DataFrame> for StrategyFilter<BooleanChunked> {
    fn from(df: DataFrame) -> Self {
        assert_eq!(df.width(), 4);
        Self {
            long_open: df.select_at_idx(0).unwrap().bool().unwrap().clone(),
            long_stop: df.select_at_idx(1).unwrap().bool().unwrap().clone(),
            short_open: df.select_at_idx(2).unwrap().bool().unwrap().clone(),
            short_stop: df.select_at_idx(3).unwrap().bool().unwrap().clone(),
        }
    }
}

#[cfg(feature = "polars")]
impl<'a> From<&'a DataFrame> for StrategyFilter<&'a BooleanChunked> {
    fn from(df: &'a DataFrame) -> Self {
        assert_eq!(df.width(), 4);
        Self {
            long_open: df.select_at_idx(0).unwrap().bool().unwrap(),
            long_stop: df.select_at_idx(1).unwrap().bool().unwrap(),
            short_open: df.select_at_idx(2).unwrap().bool().unwrap(),
            short_stop: df.select_at_idx(3).unwrap().bool().unwrap(),
        }
    }
}
