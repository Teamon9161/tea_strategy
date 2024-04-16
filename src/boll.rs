#![allow(clippy::unused_unit)]
use crate::StrategyFilter;
use itertools::izip;
use serde::Deserialize;
use tea_core::prelude::*;
use tea_rolling::*;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct BollKwargs {
    // window, open_width, stop_width, take_profit_width
    pub params: (usize, f64, f64, Option<f64>),
    pub min_periods: Option<usize>,
    pub delay_open: bool,
    pub long_signal: f64,
    pub short_signal: f64,
    pub close_signal: f64,
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
            if $fac.is_some() && $middle.is_some() && $std.is_some() && $std.unwrap() > 0. {
                let fac = ($fac.unwrap() - $middle.unwrap()) / $std.unwrap();
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
            Some($last_signal)
        }
    };
}

#[allow(clippy::collapsible_else_if)]
pub fn boll<
    O: Vec1<Item = Option<f64>>,
    T,
    V: RollingValidFeature<T>,
    VMask: Vec1View<Item = Option<bool>>,
>(
    fac_arr: V,
    filter: Option<StrategyFilter<VMask>>,
    kwargs: &BollKwargs,
) -> O
where
    T: Number + IsNone,
{
    let m = kwargs.params.1;
    let mut last_signal = kwargs.close_signal;
    let mut last_fac = 0.;
    let min_periods = kwargs.min_periods.unwrap_or(kwargs.params.0 / 2);
    let middle_arr: O = fac_arr.ts_vmean(kwargs.params.0, Some(min_periods));
    let std_arr: O = fac_arr.ts_vstd(kwargs.params.0, Some(min_periods));
    if let Some(filter) = filter {
        let zip_ = izip!(
            fac_arr.opt_iter_cast::<f64>(),
            middle_arr.opt_iter_cast::<f64>(),
            std_arr.opt_iter_cast::<f64>(),
            filter.to_iter(),
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
                    },
                )
                .collect_trusted_vec1()
            }
        }
    } else {
        let zip_ = izip!(
            fac_arr.opt_iter_cast::<f64>(),
            middle_arr.opt_iter_cast::<f64>(),
            std_arr.opt_iter_cast::<f64>(),
        );
        if kwargs.delay_open {
            if let Some(m3) = kwargs.params.3 {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac,
                        profit_p=>m3,
                    )
                })
                .collect_trusted_vec1()
            } else {
                zip_.map(|(fac, middle, std)| {
                    boll_logic_impl!(kwargs, fac, middle, std, last_signal, last_fac,)
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
                })
                .collect_trusted_vec1()
            }
        }
    }
}
