use crate::StrategyFilter;
use serde::Deserialize;
use tea_core::prelude::*;
use tea_rolling::*;

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
    pub step: usize, // adjust step
    pub init_pos: f64,
    pub win_p_addup: f64,
    pub take_profit: f64,
    // pub stop_loss: f64,
    pub b: f64, // profit loss ratio
}

pub fn martingale<
    O: Vec1<Item = Option<f64>>,
    T,
    V: RollingValidFeature<T>,
    VMask: Vec1View<Item = Option<bool>>,
>(
    close_vec: V,
    filter: Option<StrategyFilter<VMask>>,
    kwargs: &MartingaleKwargs,
) -> O
where
    T: Number + IsNone,
{
    // let mut trades_profit = VecDeque::<f64>::new();
    let b = kwargs.b; // profit loss ratio
    let init_win_p = arc_kelly(kwargs.init_pos, b);
    let mut win_p = init_win_p; // probability of win
    let mut last_signal = kwargs.init_pos;
    let mut open_price: Option<f64> = None;
    let mut step = 0;
    let std_vec: O = close_vec.ts_vstd(kwargs.step, None);
    if let Some(_filter) = filter {
        todo!()
        // close_vec.to_iter()
        //     .zip(filters.to_iter())
        //     .map(||)
    } else {
        close_vec
            .opt_iter_cast::<f64>()
            .zip(std_vec.opt_iter_cast::<f64>())
            .map(|(close, std)| {
                if close.is_none() || std.is_none() {
                    return Some(last_signal);
                }
                let close = close.unwrap();
                let std = std.unwrap();
                step += 1;
                if step >= kwargs.step {
                    // adjust position
                    step = 0;
                    if let Some(op) = open_price {
                        let profit = close - op;
                        if profit > op + std * kwargs.take_profit {
                            // take profit and reset win probability
                            win_p -= kwargs.win_p_addup;
                            if win_p < init_win_p {
                                win_p = init_win_p;
                            }
                            last_signal = kwargs.init_pos;
                            open_price = Some(close);
                        } else if profit < op - std * kwargs.take_profit {
                            // increment win probability
                            win_p += kwargs.win_p_addup;
                            if win_p > 1. {
                                win_p = 1.;
                            }
                            last_signal = kelly(win_p, b);
                            open_price = Some(close)
                        } else {
                            // just keep position
                        }
                    } else {
                        open_price = Some(close);
                    }
                    Some(last_signal)
                } else {
                    Some(last_signal)
                }
                // 是否止盈或止损
            })
            .collect_trusted_vec1()
    }
}
