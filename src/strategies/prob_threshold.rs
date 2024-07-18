use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use crate::StrategyFilter;

#[derive(Deserialize)]
pub struct ProbThresholdKwargs {
    // long open thres, long stop thres, short open thres, short stop thres
    pub thresholds: (f64, f64, f64, f64),
    pub per_hand: f64,
    pub max_hand: f64,
}

fn check_kwargs(kwargs: &ProbThresholdKwargs) -> TResult<()> {
    tensure!(
        kwargs.per_hand <= kwargs.max_hand,
        "hand per signal should be less than or equal to max hand"
    );
    tensure!(
        kwargs.thresholds.0 > kwargs.thresholds.1,
        "long open thres should be greater than long stop thres"
    );
    tensure!(
        kwargs.thresholds.2 < kwargs.thresholds.3,
        "short open thres should be less than short stop thres"
    );
    tensure!(
        kwargs.thresholds.0 > kwargs.thresholds.2,
        "long open thres should be greater than short open thres"
    );
    // tensure!(
    //     kwargs.thresholds.1 >= kwargs.thresholds.3,
    //     "long stop thres should be greater or equal than short stop thres"
    // );
    Ok(())
}

#[allow(clippy::collapsible_else_if, clippy::if_same_then_else)]
pub fn prob_threshold<O: Vec1<U>, U, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    fac_arr: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &ProbThresholdKwargs,
) -> TResult<O>
where
    T: IsNone,
    T::Inner: Number,
    f64: Cast<U>,
{
    check_kwargs(kwargs)?;
    let mut last_signal = 0.;
    let out = if let Some(filter) = filter {
        izip!(fac_arr.titer(), filter.titer(),)
            .map(|(fac, (long_open, long_stop, short_open, short_stop))| {
                if fac.not_none() {
                    let fac = fac.unwrap().f64();
                    // open condition
                    let mut open_flag = false;
                    let long_open_cond = fac >= kwargs.thresholds.0
                        && (last_signal + kwargs.per_hand <= kwargs.max_hand)
                        && long_open.unwrap_or(true);
                    let short_open_cond = fac <= kwargs.thresholds.2
                        && (last_signal - kwargs.per_hand >= -kwargs.max_hand)
                        && short_open.unwrap_or(true);
                    if long_open_cond {
                        if last_signal >= 0. {
                            last_signal += kwargs.per_hand;
                        } else {
                            // if there is a breaking change, we should
                            // close short position and open long position
                            last_signal = kwargs.per_hand;
                        }
                        open_flag = true;
                    } else if short_open_cond {
                        if last_signal <= 0. {
                            last_signal -= kwargs.per_hand;
                        } else {
                            // if there is a breaking change, we should
                            // close long position and open short position
                            last_signal = -kwargs.per_hand;
                        }
                        open_flag = true;
                    }
                    // stop condition
                    if (!open_flag) && (last_signal != 0.) {
                        // we can skip stop condition if trade is already close or open
                        let long_stop_cond =
                            (fac <= kwargs.thresholds.1) || long_stop.unwrap_or(false);
                        let short_stop_cond =
                            (fac >= kwargs.thresholds.3) || short_stop.unwrap_or(false);
                        if (last_signal > 0.) && long_stop_cond {
                            last_signal = 0.;
                        } else if (last_signal < 0.) && short_stop_cond {
                            last_signal = 0.;
                        }
                    }
                }
                last_signal.cast()
            })
            .collect_trusted_vec1()
    } else {
        fac_arr
            .titer()
            .map(|fac| {
                if fac.not_none() {
                    let fac = fac.unwrap().f64();
                    // open condition
                    let mut open_flag = false;
                    let long_open_cond = fac >= kwargs.thresholds.0
                        && (last_signal + kwargs.per_hand <= kwargs.max_hand);
                    let short_open_cond = fac <= kwargs.thresholds.2
                        && (last_signal - kwargs.per_hand >= -kwargs.max_hand);
                    if long_open_cond {
                        if last_signal >= 0. {
                            last_signal += kwargs.per_hand;
                        } else {
                            // if there is a breaking change, we should
                            // close short position and open long position
                            last_signal = kwargs.per_hand;
                        }
                        open_flag = true;
                    } else if short_open_cond {
                        if last_signal <= 0. {
                            last_signal -= kwargs.per_hand;
                        } else {
                            // if there is a breaking change, we should
                            // close long position and open short position
                            last_signal = -kwargs.per_hand;
                        }
                        open_flag = true;
                    }
                    // stop condition
                    if (!open_flag) && (last_signal != 0.) {
                        // we can skip stop condition if trade is already close or open
                        let long_stop_cond = fac <= kwargs.thresholds.1;
                        let short_stop_cond = fac >= kwargs.thresholds.3;
                        if (last_signal > 0.) && long_stop_cond {
                            last_signal = 0.;
                        } else if (last_signal < 0.) && short_stop_cond {
                            last_signal = 0.;
                        }
                    }
                }
                last_signal.cast()
            })
            .collect_trusted_vec1()
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_prob_thres() -> TResult<()> {
        let prob_vec = vec![0.3, 0.6, 0.7, 0.6, 0.4, 0.2, 0.5, 0.4];
        let kwargs = ProbThresholdKwargs {
            thresholds: (0.6, 0.5, 0.4, 0.5),
            per_hand: 1.,
            max_hand: 2.,
        };
        let filter: Option<StrategyFilter<Vec<Option<bool>>>> = None;
        let signal: Vec<f64> = prob_threshold(&prob_vec, filter.as_ref(), &kwargs)?;
        let expect = vec![-1., 1., 2., 2., -1., -2., 0., -1.];
        assert_eq!(signal, expect);
        Ok(())
    }
}
