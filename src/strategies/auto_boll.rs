use std::collections::VecDeque;

use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use crate::StrategyFilter;

#[derive(Deserialize, Clone)]
pub struct AutoBollKwargs {
    // window, open_width, stop_width
    pub params: (usize, f64, f64),
    pub min_periods: Option<usize>,
    pub pos_map: Option<(Vec<i32>, Vec<f64>)>,
    pub delay_open: bool,
    pub long_signal: f64,
    pub short_signal: f64,
    pub close_signal: f64,
}

macro_rules! boll_logic_impl {
    (
        $kwargs: expr,
        $fac: expr, $middle: expr, $std: expr,
        $last_signal: expr, $last_fac: expr, $open_price: expr,
        $trades_profit: expr, $trade_num_vec: expr, $pos_vec: expr,
        $(filters=>($long_open: expr, $long_stop: expr, $short_open: expr, $short_stop: expr),)?
        $(long_open=>$long_open_cond: expr,)?
        $(short_open=>$short_open_cond: expr,)?
        $(,)?
    ) => {
        {
            if $fac.not_none() && $middle.not_none() && $std.not_none() && $std.clone().unwrap() > 0. {
                let ori_fac = $fac.unwrap().f64();
                let fac = (ori_fac - $middle.unwrap()) / $std.unwrap();
                // == open condition
                let mut open_flag = false;
                if ($last_signal <= $kwargs.close_signal) && (fac >= $kwargs.params.1) $(&& $long_open.unwrap_or(true))? $(&& $long_open_cond)? {
                    // long open
                    $open_price = ori_fac;
                    let mut profit_level = 0;
                    $trades_profit.iter().for_each(|profit| {
                        if *profit > 0. {
                            profit_level += 1
                        } else if *profit < 0. {
                            profit_level -= 1
                        }
                    });
                    let adjust_signal = get_adjust_param(profit_level, $trade_num_vec, $pos_vec);
                    $last_signal = $kwargs.long_signal * adjust_signal;
                    open_flag = true;
                } else if ($last_signal >= $kwargs.close_signal) && (fac <= -$kwargs.params.1) $(&& $short_open.unwrap_or(true))? $(&& $short_open_cond)? {
                    // short open
                    $open_price = ori_fac;
                    let mut profit_level = 0;
                    $trades_profit.iter().for_each(|profit| {
                        if *profit > 0. {
                            profit_level += 1
                        } else if *profit < 0. {
                            profit_level -= 1
                        }
                    });
                    let adjust_signal = get_adjust_param(profit_level, $trade_num_vec, $pos_vec);
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
                        let profit = (ori_fac / $open_price - 1.) * $last_signal;
                        $trades_profit.pop_front();
                        $trades_profit.push_back(profit);
                        $last_signal = $kwargs.close_signal;
                        $open_price = f64::NAN;
                    } else if (($last_fac < -$kwargs.params.2) && (fac >= -$kwargs.params.2))
                        $(|| $short_stop.unwrap_or(false))?
                    {
                        // short stop
                        let profit = (ori_fac / $open_price - 1.) * $last_signal;
                        $trades_profit.pop_front();
                        $trades_profit.push_back(profit);
                        $last_signal = $kwargs.close_signal;
                        $open_price = f64::NAN;
                    }
                }
                // == update open info
                $last_fac = fac;
            }
            $last_signal
        }
    };
}

fn get_adjust_param(win_time: i32, trades_num_vec: &[i32], pos_vec: &[f64]) -> f64 {
    let mut param = f64::NAN;
    trades_num_vec
        .windows(2)
        .zip(pos_vec)
        .for_each(|(bound, pos)| {
            if win_time < 0 {
                if (bound[0] < win_time) && (win_time <= bound[1]) {
                    param = *pos;
                }
            } else if (bound[0] <= win_time) && (win_time < bound[1]) {
                param = *pos;
            }
        });
    param
}

#[allow(clippy::collapsible_else_if)]
pub fn auto_boll<O: Vec1<T::Cast<f64>>, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    fac_arr: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &AutoBollKwargs,
) -> TResult<O>
where
    T: IsNone,
    T::Inner: Number,
{
    let m = kwargs.params.1;
    let mut last_signal = kwargs.close_signal;
    let mut last_fac = 0.;
    let min_periods = kwargs.min_periods.unwrap_or(kwargs.params.0 / 2);
    let max_trades_num = kwargs
        .pos_map
        .as_ref()
        // .map(|pm| pm.0.iter().map(|v| v.abs()).max().unwrap_or(0))
        .map(|pm| AggBasic::max(pm.0.titer().abs()).unwrap())
        .unwrap_or(3) as usize;
    // get and check pos_map
    let (mut trades_num_vec, pos_vec) = kwargs
        .pos_map
        .clone()
        .unwrap_or((vec![-4, -2, 2], vec![1., 0.75, 0.5, 0.25]));
    // assert!(!pos_vec.is_empty());
    tensure!(!pos_vec.is_empty(), "pos vec should not be empty");
    tensure!(
        trades_num_vec.len() + 1 == pos_vec.len(),
        "trades num vec length should be pos vec length - 1"
    );
    // assert!(Vec1ViewAgg::min(pos_vec.titer()).unwrap().f64() >= -1.);
    // assert!(Vec1ViewAgg::max(pos_vec.titer()).unwrap().f64() <= 1.);
    trades_num_vec.insert(0, i32::MIN);
    trades_num_vec.push(i32::MAX);

    let middle_arr: Vec<f64> = fac_arr.ts_vmean(kwargs.params.0, Some(min_periods));
    let std_arr: Vec<f64> = fac_arr.ts_vstd(kwargs.params.0, Some(min_periods));
    let mut open_price = f64::NAN;
    let mut trades_profit: VecDeque<f64> = vec![0.; max_trades_num].into();
    let out = if let Some(filter) = filter {
        let zip_ = izip!(
            fac_arr.titer(),
            middle_arr.titer(),
            std_arr.titer(),
            filter.titer(),
        );
        if kwargs.delay_open {
            zip_.map(
                |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                    T::inner_cast(boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac, open_price,
                        trades_profit, &trades_num_vec, &pos_vec,
                        filters=>(long_open, long_stop, short_open, short_stop),
                    ))
                },
            )
            .collect_trusted_vec1()
        } else {
            zip_.map(
                |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                    T::inner_cast(boll_logic_impl!(
                        kwargs, fac, middle, std,
                        last_signal, last_fac, open_price,
                        trades_profit, &trades_num_vec, &pos_vec,
                        filters=>(long_open, long_stop, short_open, short_stop),
                        long_open=>last_fac < m,
                        short_open=>last_fac > -m,
                    ))
                },
            )
            .collect_trusted_vec1()
        }
    } else {
        let zip_ = izip!(fac_arr.titer(), middle_arr.titer(), std_arr.titer(),);
        if kwargs.delay_open {
            zip_.map(|(fac, middle, std)| {
                T::inner_cast(boll_logic_impl!(
                    kwargs,
                    fac,
                    middle,
                    std,
                    last_signal,
                    last_fac,
                    open_price,
                    trades_profit,
                    &trades_num_vec,
                    &pos_vec,
                ))
            })
            .collect_trusted_vec1()
        } else {
            zip_.map(|(fac, middle, std)| {
                T::inner_cast(boll_logic_impl!(
                    kwargs, fac, middle, std,
                    last_signal, last_fac, open_price,
                    trades_profit, &trades_num_vec, &pos_vec,
                    long_open=>last_fac < m,
                    short_open=>last_fac > -m,
                ))
            })
            .collect_trusted_vec1()
        }
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_auto_boll() -> TResult<()> {
        let close = vec![
            10., 11., 11.9, 10., 11., 12., 10., 11., 12., 13., 14., 10., 7., 5., 4., 3., 4., 4.,
            3., 2.,
        ];
        let kwargs = AutoBollKwargs {
            params: (4, 1.0, 0.),
            min_periods: None,
            pos_map: None,
            delay_open: false,
            long_signal: 1.0,
            short_signal: -1.0,
            close_signal: 0.0,
        };
        let filter: Option<StrategyFilter<Vec<Option<bool>>>> = None;
        let signal: Vec<_> = auto_boll(&close, filter.as_ref(), &kwargs)?;
        let expect: Vec<_> = vec![
            0., 0., 0., 0., 0., 0., 0., 0., 0., 0.5, 0.5, -0.5, -0.5, -0.5, -0.5, -0.5, 0., 0., 0.,
            -0.5,
        ];
        assert_eq!(expect, signal);
        Ok(())
    }
}
