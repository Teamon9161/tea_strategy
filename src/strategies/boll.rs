use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use crate::StrategyFilter;

#[derive(Deserialize, Clone)]
pub struct BollKwargs {
    /// window, open_width, stop_width, take_profit_width
    pub params: (usize, f64, f64, Option<f64>),
    pub min_periods: Option<usize>,
    pub delay_open: bool,
    /// if false, use 0 as middle, 1 as std
    pub zscore: bool,
    pub long_signal: f64,
    pub short_signal: f64,
    pub close_signal: f64,
}

impl Default for BollKwargs {
    fn default() -> Self {
        Self {
            params: (20, 0., 0., None),
            min_periods: None,
            delay_open: true,
            zscore: true,
            long_signal: 1.0,
            short_signal: -1.0,
            close_signal: 0.0,
        }
    }
}

impl BollKwargs {
    #[inline]
    pub fn new(n: usize, m: f64) -> Self {
        Self {
            params: (n, m, 0., None),
            ..Default::default()
        }
    }

    #[inline]
    pub fn with_m(mut self, m: f64) -> Self {
        self.params.1 = m;
        self
    }
}

macro_rules! boll_logic_impl {
    (
        $kwargs: expr,
        $fac: expr, $middle: expr, $std: expr,
        $last_signal: expr, $last_fac: expr,
        $(filters=>($long_open: expr, $long_stop: expr, $short_open: expr, $short_stop: expr),)?
        $(long_open=>$long_open_cond: expr,)?
        $(short_open=>$short_open_cond: expr,)?
        $(profit_p=>$m3: expr)?
        $(,)?
    ) => {
        {
            if $fac.not_none() && $middle.not_none() && $std.not_none() && $std.clone().unwrap() > 0. {
                let ori_fac = $fac.unwrap().f64();
                let fac = (ori_fac - $middle.unwrap()) / $std.unwrap();
                // == open condition
                let mut open_flag = false;
                if ($last_signal != $kwargs.long_signal) && (fac >= $kwargs.params.1) $(&& $long_open.unwrap_or(true))? $(&& $long_open_cond)? {
                    // long open
                    $last_signal = $kwargs.long_signal;
                    open_flag = true;
                } else if ($last_signal != $kwargs.short_signal) && (fac <= -$kwargs.params.1) $(&& $short_open.unwrap_or(true))? $(&& $short_open_cond)? {
                    // short open
                    $last_signal = $kwargs.short_signal;
                    open_flag = true;
                }
                // == stop condition
                if (!open_flag) && ($last_signal != $kwargs.close_signal) {
                    // we can skip stop condition if trade is already close or open
                    if (($last_fac > $kwargs.params.2) && (fac <= $kwargs.params.2))
                        $(|| $long_stop.unwrap_or(false))?  // additional stop condition
                        $(|| fac >= $m3)?  // profit stop condition
                    {
                        // long stop
                        $last_signal = $kwargs.close_signal;
                    } else if (($last_fac < -$kwargs.params.2) && (fac >= -$kwargs.params.2))
                        $(|| $short_stop.unwrap_or(false))?
                        $(|| fac <= -$m3)?  // profit stop condition
                    {
                        // short stop
                        $last_signal = $kwargs.close_signal;
                    }
                }
                // == update open info
                $last_fac = fac;
            }
            $last_signal
        }
    };
}

#[allow(clippy::collapsible_else_if)]
pub fn boll<O: Vec1<T::Cast<f64>>, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    fac_arr: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &BollKwargs,
) -> O
where
    T: IsNone,
    T::Inner: Number,
{
    let m = kwargs.params.1;
    let mut last_signal = kwargs.close_signal;
    let mut last_fac = 0.;
    let min_periods = kwargs.min_periods.unwrap_or(kwargs.params.0 / 2);
    let (middle_arr, std_arr) = if kwargs.zscore {
        let middle_arr: Vec<f64> = fac_arr.ts_vmean(kwargs.params.0, Some(min_periods));
        let std_arr: Vec<f64> = fac_arr.ts_vstd(kwargs.params.0, Some(min_periods));
        (middle_arr, std_arr)
    } else {
        (vec![0.; fac_arr.len()], vec![1.; fac_arr.len()])
    };
    if let Some(filter) = filter {
        let zip_ = izip!(
            fac_arr.titer(),
            middle_arr.titer(),
            std_arr.titer(),
            filter.titer(),
        );
        if kwargs.delay_open {
            if let Some(m3) = kwargs.params.3 {
                zip_.map(
                    |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                        boll_logic_impl!(
                            kwargs, fac, middle, std,
                            last_signal, last_fac,
                            filters=>(long_open, long_stop, short_open, short_stop),
                            profit_p=>m3,
                        )
                        .into_cast::<T>()
                    },
                )
                .collect_trusted_vec1()
            } else {
                zip_.map(
                    |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                        boll_logic_impl!(
                            kwargs, fac, middle, std,
                            last_signal, last_fac,
                            filters=>(long_open, long_stop, short_open, short_stop),
                        )
                        .into_cast::<T>()
                    },
                )
                .collect_trusted_vec1()
            }
        } else {
            if let Some(m3) = kwargs.params.3 {
                zip_.map(
                    |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                        boll_logic_impl!(
                            kwargs, fac, middle, std,
                            last_signal, last_fac,
                            filters=>(long_open, long_stop, short_open, short_stop),
                            long_open=>last_fac < m,
                            short_open=>last_fac > -m,
                            profit_p=>m3,
                        )
                        .into_cast::<T>()
                    },
                )
                .collect_trusted_vec1()
            } else {
                zip_.map(
                    |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                        boll_logic_impl!(
                            kwargs, fac, middle, std,
                            last_signal, last_fac,
                            filters=>(long_open, long_stop, short_open, short_stop),
                            long_open=>last_fac < m,
                            short_open=>last_fac > -m,
                        )
                        .into_cast::<T>()
                    },
                )
                .collect_trusted_vec1()
            }
        }
    } else {
        let zip_ = izip!(fac_arr.titer(), middle_arr.titer(), std_arr.titer(),);
        if kwargs.delay_open {
            if let Some(m3) = kwargs.params.3 {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac,
                        profit_p=>m3,
                    )
                    .into_cast::<T>()
                })
                .collect_trusted_vec1()
            } else {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(kwargs, fac, middle, std, last_signal, last_fac,)
                        .into_cast::<T>()
                })
                .collect_trusted_vec1()
            }
        } else {
            if let Some(m3) = kwargs.params.3 {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac,
                        long_open=>last_fac < m,
                        short_open=>last_fac > -m,
                        profit_p=>m3,
                    )
                    .into_cast::<T>()
                })
                .collect_trusted_vec1()
            } else {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac,
                        long_open=>last_fac < m,
                        short_open=>last_fac > -m,
                    )
                    .into_cast::<T>()
                })
                .collect_trusted_vec1()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_boll() {
        let close = vec![
            10., 11., 11.9, 10., 11., 12., 10., 11., 12., 13., 14., 10., 7., 5., 4., 3., 4., 4.,
            3., 2.,
        ];
        let kwargs = BollKwargs {
            params: (4, 1.0, 0., None),
            min_periods: None,
            delay_open: false,
            ..Default::default()
        };
        let filter: Option<StrategyFilter<Vec<Option<bool>>>> = None;
        let signal: Vec<_> = boll(&close.opt(), filter.as_ref(), &kwargs);
        let expect: Vec<_> = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, -1, -1, -1, -1, -1, 0, 0, 0, -1,
        ]
        .opt()
        .opt_iter_cast::<f64>()
        .collect_trusted_vec1();
        assert_eq!(expect, signal);
    }
}
