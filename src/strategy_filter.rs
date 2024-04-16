use itertools::izip;
use tea_core::prelude::*;

pub struct StrategyFilter<T: Vec1View<Item = Option<bool>>> {
    pub long_open: T,
    pub long_stop: T,
    pub short_open: T,
    pub short_stop: T,
}

pub type FilterElement = (Option<bool>, Option<bool>, Option<bool>, Option<bool>);


impl<T: Vec1View<Item = Option<bool>>> StrategyFilter<T> {
    pub fn to_iter(
        &self,
    ) -> TrustIter<impl Iterator<Item = FilterElement> + '_, FilterElement> {
        let iter = izip!(
            self.long_open.to_iter(),
            self.long_stop.to_iter(),
            self.short_open.to_iter(),
            self.short_stop.to_iter()
        );
        TrustIter::new(iter, self.long_open.len())
    }
}
// impl<'a> StrategyFilter<'a> {
//     fn from_inputs(inputs: &'a [Series], idxs: (usize, usize, usize, usize)) -> PolarsResult<Self> {
//         Ok(Self {
//             long_open: inputs[idxs.0].bool()?,
//             long_stop: inputs[idxs.1].bool()?,
//             short_open: inputs[idxs.2].bool()?,
//             short_stop: inputs[idxs.3].bool()?,
//         })
//     }
// }
