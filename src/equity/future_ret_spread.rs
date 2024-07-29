use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

use super::CommissionType;

#[derive(Deserialize)]
pub struct FutureRetSpreadKwargs {
    pub init_cash: usize,
    pub multiplier: f64,
    pub leverage: f64,
    pub c_rate: f64,
    pub blowup: bool,
    pub commission_type: CommissionType,
}

pub fn calc_future_ret_with_spread<O, T, V, VMask>(
    pos_vec: &V,
    open_vec: &V,
    close_vec: &V,
    spread_vec: &V,
    contract_chg_signal_vec: Option<VMask>,
    kwargs: &FutureRetSpreadKwargs,
) -> O
where
    T: IsNone,
    T::Inner: Number,
    V: Vec1View<T>,
    VMask: Vec1View<Option<bool>>,
    O: Vec1<T::Cast<f64>>,
{
    let mut cash = kwargs.init_cash as f64;
    let mut last_pos = 0_f64; // pos_arr[0];
    let mut last_lot_num = 0.;
    if pos_vec.is_empty() {
        return O::empty();
    }
    let mut last_close = None;
    let blowup = kwargs.blowup;
    let multiplier = kwargs.multiplier;
    let commission_type = kwargs.commission_type;
    let leverage = kwargs.leverage;
    let c_rate = kwargs.c_rate;
    if let Some(contract_chg_signal_vec) = contract_chg_signal_vec {
        izip!(
            pos_vec.titer(),
            open_vec.titer(),
            close_vec.titer(),
            spread_vec.titer(),
            contract_chg_signal_vec.titer(),
        )
        .map(|(pos, open, close, spread, chg)| {
            if pos.is_none() || open.is_none() || close.is_none() {
                return cash.into_cast::<T>();
            } else if blowup && cash <= 0. {
                return 0_f64.into_cast::<T>();
            }
            let pos = pos.unwrap().f64();
            let open = open.unwrap().f64();
            let close = close.unwrap().f64();
            let chg = chg.unwrap();
            if last_close.is_none() {
                last_close = Some(open)
            }
            if (last_lot_num != 0.) && (!chg) {
                // do not calculate the profit and loss of the jump open when there is a contract change
                cash +=
                    last_lot_num * (open - last_close.unwrap()) * multiplier * last_pos.signum();
            }
            // we use pos to determine the position change, so leverage must be a constant
            if (pos != last_pos) || chg {
                // the position has changed, calculate the new theoretical number of lots
                let l = ((cash * leverage * pos.abs()) / (multiplier * open)).floor();
                let (lot_num, lot_num_change) = if !chg {
                    (
                        l,
                        (l * pos.signum() - last_lot_num * last_pos.signum()).abs(),
                    )
                } else {
                    (l, l.abs() * 2.)
                };
                // addup the commision fee
                if let CommissionType::Percent = commission_type {
                    let open_mul_c_rate = open * c_rate;
                    let spread = if spread.is_none() {
                        open_mul_c_rate
                    } else {
                        spread.unwrap().f64()
                    };
                    cash -= lot_num_change * multiplier * (open_mul_c_rate + spread);
                } else {
                    let spread = if spread.is_none() {
                        c_rate
                    } else {
                        spread.unwrap().f64() * multiplier
                    };
                    cash -= lot_num_change * (c_rate + spread);
                };
                // update last lot num and last pos
                last_lot_num = lot_num;
                last_pos = pos;
            }
            // calculate the profit and loss of the current period
            if last_lot_num != 0. {
                cash += last_lot_num * last_pos.signum() * (close - open) * multiplier;
            }
            last_close = Some(close); // update last close
            cash.into_cast::<T>()
        })
        .collect_trusted_vec1()
    } else {
        // ignore contract chg signal
        // this should be faster than the above
        izip!(
            pos_vec.titer(),
            open_vec.titer(),
            close_vec.titer(),
            spread_vec.titer(),
        )
        .map(|(pos, open, close, spread)| {
            if pos.is_none() || open.is_none() || close.is_none() {
                return cash.into_cast::<T>();
            } else if blowup && cash <= 0. {
                return 0_f64.into_cast::<T>();
            }
            let pos = pos.unwrap().f64();
            let open = open.unwrap().f64();
            let close = close.unwrap().f64();
            if last_close.is_none() {
                last_close = Some(open)
            }
            if last_lot_num != 0. {
                // do not calculate the profit and loss of the jump open when there is a contract change
                cash +=
                    last_lot_num * (open - last_close.unwrap()) * multiplier * last_pos.signum();
            }
            // we use pos to determine the position change, so leverage must be a constant
            if pos != last_pos {
                // the position has changed, calculate the new theoretical number of lots
                let l = ((cash * leverage * pos.abs()) / (multiplier * open)).floor();
                let (lot_num, lot_num_change) = (
                    l,
                    (l * pos.signum() - last_lot_num * last_pos.signum()).abs(),
                );
                // addup the commision fee
                if let CommissionType::Percent = commission_type {
                    let open_mul_c_rate = open * c_rate;
                    let spread = if spread.is_none() {
                        open_mul_c_rate
                    } else {
                        spread.unwrap().f64()
                    };
                    cash -= lot_num_change * multiplier * (open_mul_c_rate + spread);
                } else {
                    let spread = if spread.is_none() {
                        c_rate
                    } else {
                        spread.unwrap().f64() * multiplier
                    };
                    cash -= lot_num_change * (c_rate + spread);
                };
                // update last lot num and last pos
                last_lot_num = lot_num;
                last_pos = pos;
            }
            // calculate the profit and loss of the current period
            if last_lot_num != 0. {
                cash += last_lot_num * last_pos.signum() * (close - open) * multiplier;
            }
            last_close = Some(close); // update last close
            cash.into_cast::<T>()
        })
        .collect_trusted_vec1()
    }
}
