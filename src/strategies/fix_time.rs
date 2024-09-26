use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use crate::StrategyFilter;

#[derive(Deserialize, Clone)]
pub struct FixTimeKwargs {
    pub n: usize,
    pub pos_map: Option<(Vec<f64>, Vec<f64>)>,
    pub extend_time: bool,
}

fn get_pos(fac: f64, bound_vec: &[f64], pos_vec: &Vec<f64>) -> f64 {
    let mut param = f64::NAN;
    bound_vec.windows(2).zip(pos_vec).for_each(|(bound, pos)| {
        if fac < 0. {
            if (bound[0] < fac) && (fac <= bound[1]) {
                param = *pos;
            }
        } else if (bound[0] <= fac) && (fac < bound[1]) {
            param = *pos;
        }
    });
    param
}

#[allow(clippy::collapsible_else_if)]
pub fn fix_time<O: Vec1<T::Cast<f64>>, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    fac_arr: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &FixTimeKwargs,
) -> TResult<O>
where
    T: IsNone,
    T::Inner: Number,
{
    let (mut bound_vec, pos_vec) = kwargs
        .pos_map
        .clone()
        .unwrap_or((vec![-1.5, 1.5], vec![-1., 0., 1.]));
    tensure!(!pos_vec.is_empty(), "pos vec should not be empty");
    tensure!(
        bound_vec.len() + 1 == pos_vec.len(),
        "bound vec length should be pos vec length - 1"
    );
    tensure!(kwargs.n >= 1, "n should be greater than or equal to 1");
    bound_vec.insert(0, f64::MIN);
    bound_vec.push(f64::MAX);
    let mut remain_period = 0;
    let mut last_signal = 0.;
    let out = if let Some(filter) = filter {
        izip!(fac_arr.titer(), filter.titer(),)
            .map(|(fac, (long_open, long_stop, short_open, short_stop))| {
                if remain_period >= 1 {
                    remain_period -= 1;
                }
                if remain_period == 0 {
                    last_signal = 0.;
                }
                if fac.not_none() {
                    let new_signal = get_pos(fac.unwrap().f64(), &bound_vec, &pos_vec);
                    if new_signal != 0. {
                        last_signal = new_signal;
                        if remain_period == 0 || kwargs.extend_time {
                            remain_period = kwargs.n;
                        }
                    } else if remain_period == 0 {
                        last_signal = 0.;
                    }
                }
                // process long_open, long_stop filter
                if (last_signal > 0.)
                    && ((!long_open.unwrap_or(true)) || long_stop.unwrap_or(false))
                {
                    last_signal = 0.;
                }
                // process short_open, short_stop filter
                if (last_signal < 0.)
                    && ((!short_open.unwrap_or(true)) || short_stop.unwrap_or(false))
                {
                    last_signal = 0.;
                }
                last_signal.into_cast::<T>()
            })
            .collect_trusted_vec1()
    } else {
        fac_arr
            .titer()
            .map(|fac| {
                if remain_period >= 1 {
                    remain_period -= 1;
                }
                if remain_period == 0 {
                    last_signal = 0.;
                }
                if fac.not_none() {
                    let new_signal = get_pos(fac.unwrap().f64(), &bound_vec, &pos_vec);
                    if new_signal != 0. {
                        last_signal = new_signal;
                        if remain_period == 0 || kwargs.extend_time {
                            remain_period = kwargs.n;
                        }
                    } else if remain_period == 0 {
                        last_signal = 0.;
                    }
                }
                last_signal.into_cast::<T>()
            })
            .collect_trusted_vec1()
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fix_time() -> TResult<()> {
        let fac_vec = vec![
            0.8, 0.9, 1.5, 1.6, 1.3, 1.0, 0.6, 0.3, -0.1, -0.5, -1.0, -1.6, -1.8, -1.4, -1.3, -1.3,
            -1.5,
        ];
        // test without extend_time
        let kwargs = FixTimeKwargs {
            n: 3,
            pos_map: Some((vec![-1.5, 1.5], vec![-1., 0., 1.])),
            extend_time: false,
        };
        let filter: Option<StrategyFilter<Vec<Option<bool>>>> = None;
        let signal: Vec<_> = fix_time(&fac_vec, filter.as_ref(), &kwargs)?;
        let expect: Vec<_> = vec![
            0., 0., 1., 1., 1., 0., 0., 0., 0., 0., 0., -1., -1., -1., 0., 0., -1.,
        ];
        assert_eq!(signal, expect);
        // test with extend_time
        let kwargs = FixTimeKwargs {
            n: 3,
            pos_map: Some((vec![-1.5, 1.5], vec![-1., 0., 1.])),
            extend_time: true,
        };
        let signal: Vec<_> = fix_time(&fac_vec, filter.as_ref(), &kwargs)?;
        let expect: Vec<_> = vec![
            0., 0., 1., 1., 1., 1., 0., 0., 0., 0., 0., -1., -1., -1., -1., -0., -1.,
        ];
        assert_eq!(signal, expect);
        Ok(())
    }
}
