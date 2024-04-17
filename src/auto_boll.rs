use std::collections::VecDeque;

use crate::StrategyFilter;
use itertools::izip;
use serde::Deserialize;
use tea_core::prelude::*;
use tea_rolling::*;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct AutoBollKwargs {
    // window, open_width, stop_width, analyse trade num
    pub params: (usize, f64, f64, i32),
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
        $last_signal: expr, $last_fac: expr, $open_price: expr, $trades_profit: expr,
        $(filters=>($long_open: expr, $long_stop: expr, $short_open: expr, $short_stop: expr),)?
        $(long_open=>$long_open_cond: expr,)?
        $(short_open=>$short_open_cond: expr,)?
        $(,)?
    ) => {
        {
            if $fac.is_some() && $middle.is_some() && $std.is_some() && $std.unwrap() > 0. {
                let fac = ($fac.unwrap() - $middle.unwrap()) / $std.unwrap();
                // == open condition
                let mut open_flag = false;
                if ($last_signal <= $kwargs.close_signal) && (fac >= $kwargs.params.1) $(&& $long_open.unwrap_or(true))? $(&& $long_open_cond)? {
                    // long open
                    $open_price = $fac.unwrap();
                    let mut profit_level = 0;
                    $trades_profit.iter().for_each(|profit| {
                        if *profit > 0. {
                            profit_level += 1
                        } else if *profit < 0. {
                            profit_level -= 1
                        }
                    });
                    let adjust_signal = if profit_level == $kwargs.params.3 {
                        1.
                    } else if profit_level == -$kwargs.params.3 {
                        0.5
                    } else {
                        0.75
                    };
                    $last_signal = $kwargs.long_signal * adjust_signal;
                    open_flag = true;
                } else if ($last_signal >= $kwargs.close_signal) && (fac <= -$kwargs.params.1) $(&& $short_open.unwrap_or(true))? $(&& $short_open_cond)? {
                    // short open
                    $open_price = $fac.unwrap();
                    let mut profit_level = 0;
                    $trades_profit.iter().for_each(|profit| {
                        if *profit > 0. {
                            profit_level += 1
                        } else if *profit < 0. {
                            profit_level -= 1
                        }
                    });
                    let adjust_signal = if profit_level == $kwargs.params.3 {
                        1.
                    } else if profit_level == -$kwargs.params.3 {
                        0.5
                    } else {
                        0.75
                    };
                    $last_signal = $kwargs.short_signal * adjust_signal;
                    open_flag = true;
                }
                // == stop condition
                if (!open_flag) && ($last_signal != $kwargs.close_signal) {
                    // we can skip stop condition if trade is already close or open
                    if (($last_fac > $kwargs.params.2) && (fac <= $kwargs.params.2))
                        $(|| $long_stop.unwrap_or(false))?  // additional stop condition
                    {
                        // long stop
                        let profit = ($fac.unwrap() / $open_price - 1.) * $last_signal;
                        $trades_profit.pop_front();
                        $trades_profit.push_back(profit);
                        $last_signal = $kwargs.close_signal;
                        $open_price = f64::NAN;
                    } else if (($last_fac < -$kwargs.params.2) && (fac >= -$kwargs.params.2))
                        $(|| $short_stop.unwrap_or(false))?
                    {
                        // short stop
                        let profit = ($fac.unwrap() / $open_price - 1.) * $last_signal;
                        $trades_profit.pop_front();
                        $trades_profit.push_back(profit);
                        $last_signal = $kwargs.close_signal;
                        $open_price = f64::NAN;
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
pub fn auto_boll<
    O: Vec1<Item = Option<f64>>,
    T,
    V: RollingValidFeature<T>,
    VMask: Vec1View<Item = Option<bool>>,
>(
    fac_arr: V,
    filter: Option<StrategyFilter<VMask>>,
    kwargs: &AutoBollKwargs,
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
    let mut open_price = f64::NAN;
    let mut trades_profit: VecDeque<f64> = vec![0.; kwargs.params.3 as usize].into();
    if let Some(filter) = filter {
        let zip_ = izip!(
            fac_arr.opt_iter_cast::<f64>(),
            middle_arr.opt_iter_cast::<f64>(),
            std_arr.opt_iter_cast::<f64>(),
            filter.to_iter(),
        );
        if kwargs.delay_open {
            zip_.map(
                |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac, open_price, trades_profit,
                        filters=>(long_open, long_stop, short_open, short_stop),
                    )
                },
            )
            .collect_trusted_vec1()
        } else {
            zip_.map(
                |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                    boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac, open_price, trades_profit,
                        filters=>(long_open, long_stop, short_open, short_stop),
                        long_open=>last_fac < m,
                        short_open=>last_fac > -m,
                    )
                },
            )
            .collect_trusted_vec1()
        }
    } else {
        let zip_ = izip!(
            fac_arr.opt_iter_cast::<f64>(),
            middle_arr.opt_iter_cast::<f64>(),
            std_arr.opt_iter_cast::<f64>(),
        );
        if kwargs.delay_open {
            zip_.map(|(fac, middle, std)| {
                boll_logic_impl!(
                    kwargs,
                    fac,
                    middle,
                    std,
                    last_signal,
                    last_fac,
                    open_price,
                    trades_profit,
                )
            })
            .collect_trusted_vec1()
        } else {
            zip_.map(|(fac, middle, std)| {
                boll_logic_impl!(
                    kwargs, fac, middle, std,
                    last_signal, last_fac, open_price, trades_profit,
                    long_open=>last_fac < m,
                    short_open=>last_fac > -m,
                )
            })
            .collect_trusted_vec1()
        }
    }
}