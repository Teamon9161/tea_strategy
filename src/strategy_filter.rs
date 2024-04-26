use itertools::izip;
use tevec::prelude::*;

pub struct StrategyFilter<T: Vec1View<Item = Option<bool>>> {
    pub long_open: T,
    pub long_stop: T,
    pub short_open: T,
    pub short_stop: T,
}

pub type FilterElement = (Option<bool>, Option<bool>, Option<bool>, Option<bool>);

impl<T: Vec1View<Item = Option<bool>>> StrategyFilter<T> {
    pub fn to_iter(&self) -> TrustIter<impl Iterator<Item = FilterElement> + '_> {
        let iter = izip!(
            self.long_open.to_iter(),
            self.long_stop.to_iter(),
            self.short_open.to_iter(),
            self.short_stop.to_iter()
        );
        TrustIter::new(iter, self.long_open.len())
    }
}
