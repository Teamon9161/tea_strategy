use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use crate::StrategyFilter;

#[inline]
fn kelly(p: f64, b: f64) -> f64 {
    (p * b - (1. - p)) / b
}

#[inline]
/// get win probability by current position
fn arc_kelly(pos: f64, b: f64) -> f64 {
    (pos * b + 1.) / (b + 1.)
}

#[derive(Deserialize)]
pub struct MartingaleKwargs {
    pub n: usize,            // rolling window
    pub step: Option<usize>, // adjust step
    pub init_pos: f64,
    pub win_p_addup: Option<f64>,
    pub pos_mul: Option<f64>,
    pub take_profit: f64,
    // pub stop_loss: f64,
    pub b: f64, // profit loss ratio
    pub stop_loss_m: Option<f64>,
}

pub fn martingale<O: Vec1<T::Cast<f64>>, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    close_vec: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &MartingaleKwargs,
) -> TResult<O>
where
    T::Inner: Number,
    T: IsNone,
{
    let b = kwargs.b; // profit loss ratio
    let init_win_p = arc_kelly(kwargs.init_pos, b);
    tensure!(
        (kwargs.win_p_addup.is_some() || kwargs.pos_mul.is_some())
            && !(kwargs.win_p_addup.is_some() && kwargs.pos_mul.is_some()),
        "win_p_addup and pos_mul should be exclusive"
    );
    let win_p_flag = kwargs.win_p_addup.is_some();
    let mut win_p = init_win_p; // probability of win
    let mut last_signal = kwargs.init_pos;
    let mut open_price: Option<f64> = None;
    let mut current_step = 0;
    // let middle_vec: O = close_vec.ts_vmean(kwargs.n, None);
    let std_vec: Vec<f64> = close_vec.ts_vstd(kwargs.n, None);
    let step = kwargs.step.unwrap_or(1);
    let out = if let Some(filter) = filter {
        izip!(close_vec.titer(), std_vec.titer(), filter.titer(),)
            .map(|(close, std, (long_open, _, _, _))| {
                if close.is_none() | std.is_none() {
                    return last_signal.into_cast::<T>();
                }
                let close = close.unwrap().f64();
                let std = std.unwrap().f64();
                current_step += 1;
                if current_step >= step {
                    // adjust position
                    current_step = 0;
                    if let Some(op) = open_price {
                        let profit = close - op;
                        if let Some(long_open) = long_open {
                            if !long_open {
                                // stop loss in downtrend
                                win_p = init_win_p;
                                last_signal = 0.;
                                open_price = Some(close);
                                return 0f64.into_cast::<T>();
                            }
                        }
                        if profit > std * kwargs.take_profit {
                            // take profit and reset win probability
                            win_p = init_win_p;
                            last_signal = kwargs.init_pos;
                            open_price = Some(close);
                        } else if profit < -std * kwargs.take_profit {
                            // increment win probability
                            if win_p_flag {
                                win_p += kwargs.win_p_addup.unwrap();
                                if win_p > 1. {
                                    win_p = 1.;
                                }
                                last_signal = kelly(win_p, b);
                            } else {
                                if last_signal != 0. {
                                    last_signal *= kwargs.pos_mul.unwrap();
                                } else {
                                    // in this case, we just finish stop loss
                                    // in downtrend
                                    last_signal = kwargs.init_pos;
                                }

                                if last_signal > 1. {
                                    last_signal = 1.;
                                }
                            }
                            open_price = Some(close)
                        } else {
                            // just keep position
                        }
                    } else {
                        open_price = Some(close);
                    }
                    last_signal.into_cast::<T>()
                } else {
                    last_signal.into_cast::<T>()
                }
                // 是否止盈或止损
            })
            .collect_trusted_vec1()
    } else {
        izip!(close_vec.titer(), std_vec.titer(),)
            .map(|(close, std)| {
                if close.is_none() || std.is_none() {
                    return last_signal.into_cast::<T>();
                }
                let close = close.unwrap().f64();
                let std = std.unwrap().f64();
                current_step += 1;
                if current_step >= step {
                    // adjust position
                    current_step = 0;
                    if let Some(op) = open_price {
                        let profit = close - op;
                        if profit > std * kwargs.take_profit {
                            // take profit and reset win probability
                            win_p = init_win_p;
                            last_signal = kwargs.init_pos;
                            open_price = Some(close);
                        } else if profit < -std * kwargs.take_profit {
                            // increment win probability
                            if win_p_flag {
                                win_p += kwargs.win_p_addup.unwrap();
                                if win_p > 1. {
                                    win_p = 1.;
                                }
                                last_signal = kelly(win_p, b);
                            } else {
                                if last_signal != 0. {
                                    last_signal *= kwargs.pos_mul.unwrap();
                                } else {
                                    // in this case, we just finish stop loss
                                    // in downtrend
                                    last_signal = kwargs.init_pos;
                                }

                                if last_signal > 1. {
                                    last_signal = 1.;
                                }
                            }
                            open_price = Some(close)
                        } else {
                            // just keep position
                        }
                    } else {
                        open_price = Some(close);
                    }
                    last_signal.into_cast::<T>()
                } else {
                    last_signal.into_cast::<T>()
                }
                // 是否止盈或止损
            })
            .collect_trusted_vec1()
    };
    Ok(out)
}
