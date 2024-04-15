mod boll;
use tea_core::prelude::*;

pub struct StrategyFilter<T: Vec1View<Item=Option<bool>>> {
    long_open: T,
    long_stop: T,
    short_open: T,
    short_stop: T,
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
