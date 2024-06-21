use itertools::izip;
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
