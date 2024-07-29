use itertools::izip;
use serde::{Deserialize, Deserializer};
use tevec::prelude::*;

use super::CommissionType;

#[derive(Deserialize)]
pub struct TickFutureRetKwargs {
    pub init_cash: usize,
    pub multiplier: f64,
    pub c_rate: f64,
    pub blowup: bool,
    pub commission_type: CommissionType,
    pub signal_type: SignalType,
}

#[derive(Clone, Copy)]
pub enum SignalType {
    Percent,
    Absolute,
}

impl SignalType {
    #[inline]
    pub fn parse(s: &str) -> TResult<Self> {
        match s.to_lowercase().as_str() {
            "percent" | "pct" => Ok(SignalType::Percent),
            "fixed" | "absolute" | "fix" => Ok(SignalType::Absolute),
            _ => tbail!("invalid signal type"),
        }
    }
}

impl<'de> Deserialize<'de> for SignalType {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SignalType::parse(s.as_str()).map_err(serde::de::Error::custom)
    }
}

pub fn calc_tick_future_ret<O, T, V, VMask>(
    signal_vec: &V,
    bid_vec: &V,
    ask_vec: &V,
    contract_chg_signal_vec: Option<&VMask>,
    kwargs: &TickFutureRetKwargs,
) -> O
where
    T: IsNone,
    T::Inner: Number,
    V: Vec1View<T>,
    VMask: Vec1View<Option<bool>>,
    O: Vec1<T::Cast<f64>>,
{
    if signal_vec.is_empty() {
        return O::empty();
    }
    let mut cash = kwargs.init_cash as f64;
    let mut last_lot_num = 0.;
    let mut last_chg = false;
    let mut last_mid = f64::NAN;
    let blowup = kwargs.blowup;
    let multiplier = kwargs.multiplier;
    let commission_type = kwargs.commission_type;
    let c_rate = kwargs.c_rate;
    if let SignalType::Percent = kwargs.signal_type {
        let mut last_signal = 0_f64;
        if let Some(contract_chg_signal_vec) = contract_chg_signal_vec {
            izip!(
                signal_vec.titer(),
                bid_vec.titer(),
                ask_vec.titer(),
                contract_chg_signal_vec.titer(),
            )
            .map(|(signal, bid, ask, chg)| {
                if signal.is_none() || bid.is_none() || ask.is_none() {
                    return cash.into_cast::<T>();
                } else if blowup && cash <= 0. {
                    return 0_f64.into_cast::<T>();
                }
                let signal = signal.unwrap().f64();
                let bid = bid.unwrap().f64();
                let ask = ask.unwrap().f64();
                let chg = chg.unwrap_or(false);
                let mid = (bid + ask) * 0.5;

                // update last_lot_num if contract has changed
                if last_chg {
                    // TODO(Teamon): change
                    last_lot_num = ((last_lot_num * last_mid) / mid).floor();
                }

                // calculate the profit and loss of the current period
                // we should not calculate the profit in this way if the contract has changed
                if (last_lot_num != 0.) && last_mid.not_none() && (!last_chg) {
                    cash += last_lot_num * last_signal.signum() * (mid - last_mid) * multiplier;
                }
                let out = cash;
                // TODO(Teamon): how to handle the case when the contract has changed
                // should we pass daily open price as another input?
                // currently we just ignore the profit and loss in the first tick when there is a contract change

                // addup the commision fee
                if (signal != last_signal) || chg {
                    // the position has changed, calculate the new theoretical number of lots
                    let lot_num = ((cash * signal.abs()) / (multiplier * mid)).floor();
                    if !chg {
                        let lot_num_change =
                            lot_num * signal.signum() - last_lot_num * last_signal.signum();
                        let (open_price, spread) = if lot_num_change > 0. {
                            (ask, ask - mid)
                        } else {
                            (bid, mid - bid)
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                        }
                    } else {
                        // for simple, assume spread is (bid - ask) / 2
                        // otherwise we need the bid and ask of next hot future
                        if let CommissionType::Percent = commission_type {
                            cash -= 2. * lot_num * multiplier * (mid * c_rate + (ask - bid) * 0.5);
                        } else {
                            cash -= 2. * lot_num * (c_rate + (ask - bid) * 0.5 * multiplier);
                        }
                    };

                    // update last lot num and last pos
                    last_lot_num = lot_num;
                    last_signal = signal;
                }

                last_mid = mid; // update last close
                last_chg = chg;
                out.into_cast::<T>()
            })
            .collect_trusted_vec1()
        } else {
            // ignore contract chg signal
            // this should be faster than the above
            izip!(signal_vec.titer(), bid_vec.titer(), ask_vec.titer(),)
                .map(|(signal, bid, ask)| {
                    if signal.is_none() || bid.is_none() || ask.is_none() {
                        return cash.into_cast::<T>();
                    } else if blowup && cash <= 0. {
                        return 0_f64.into_cast::<T>();
                    }
                    let signal = signal.unwrap().f64();
                    let bid = bid.unwrap().f64();
                    let ask = ask.unwrap().f64();
                    let mid = (bid + ask) * 0.5;

                    // calculate the profit and loss of the current period
                    if (last_lot_num != 0.) && last_mid.not_none() {
                        cash += last_lot_num * last_signal.signum() * (mid - last_mid) * multiplier;
                    }

                    // addup the commision fee
                    if signal != last_signal {
                        // the position has changed, calculate the new theoretical number of lots
                        let lot_num = ((cash * signal.abs()) / (multiplier * mid)).floor();
                        let lot_num_change =
                            lot_num * signal.signum() - last_lot_num * last_signal.signum();
                        let (open_price, spread) = if lot_num_change > 0. {
                            (ask, ask - mid)
                        } else {
                            (bid, mid - bid)
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                        };
                        // update last lot num and last pos
                        last_lot_num = lot_num;
                        last_signal = signal;
                    }

                    last_mid = mid; // update last close
                    cash.into_cast::<T>()
                })
                .collect_trusted_vec1()
        }
    } else {
        // absolute signal type
        if let Some(contract_chg_signal_vec) = contract_chg_signal_vec {
            izip!(
                signal_vec.titer(),
                bid_vec.titer(),
                ask_vec.titer(),
                contract_chg_signal_vec.titer(),
            )
            .map(|(lot_num, bid, ask, chg)| {
                if lot_num.is_none() || bid.is_none() || ask.is_none() {
                    return cash.into_cast::<T>();
                } else if blowup && cash <= 0. {
                    return 0_f64.into_cast::<T>();
                }
                let lot_num = lot_num.unwrap().f64();
                let bid = bid.unwrap().f64();
                let ask = ask.unwrap().f64();
                let chg = chg.unwrap_or(false);
                let mid = (bid + ask) * 0.5;

                // calculate the profit and loss of the current period
                // we should not calculate the profit in this way if the contract has changed
                if (last_lot_num != 0.) && last_mid.not_none() && (!last_chg) {
                    cash += last_lot_num * (mid - last_mid) * multiplier;
                }
                let out = cash;
                // TODO(Teamon): how to handle the case when the contract has changed
                // should we pass daily open price as another input?
                // currently we just ignore the profit and loss in the first tick when there is a contract change

                // addup the commision fee
                if (lot_num != last_lot_num) || chg {
                    if !chg {
                        let lot_num_change = lot_num - last_lot_num;
                        let (open_price, spread) = if lot_num_change > 0. {
                            (ask, ask - mid)
                        } else {
                            (bid, mid - bid)
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                        }
                    } else {
                        // for simple, assume spread is (bid - ask) / 2
                        // otherwise we need the bid and ask of next hot future
                        if let CommissionType::Percent = commission_type {
                            cash -= 2.
                                * lot_num.abs()
                                * multiplier
                                * (mid * c_rate + (ask - bid) * 0.5);
                        } else {
                            cash -= 2. * lot_num.abs() * (c_rate + (ask - bid) * 0.5 * multiplier);
                        }
                    };

                    // update last lot num and last pos
                    last_lot_num = lot_num;
                }

                last_mid = mid; // update last close
                last_chg = chg;
                out.into_cast::<T>()
            })
            .collect_trusted_vec1()
        } else {
            // ignore contract chg signal
            // this should be faster than the above
            izip!(signal_vec.titer(), bid_vec.titer(), ask_vec.titer(),)
                .map(|(lot_num, bid, ask)| {
                    if lot_num.is_none() || bid.is_none() || ask.is_none() {
                        return cash.into_cast::<T>();
                    } else if blowup && cash <= 0. {
                        return 0_f64.into_cast::<T>();
                    }
                    let lot_num = lot_num.unwrap().f64();
                    let bid = bid.unwrap().f64();
                    let ask = ask.unwrap().f64();
                    let mid = (bid + ask) * 0.5;

                    // calculate the profit and loss of the current period
                    if (last_lot_num != 0.) && last_mid.not_none() {
                        cash += last_lot_num * (mid - last_mid) * multiplier;
                    }

                    // addup the commision fee
                    if lot_num != last_lot_num {
                        let lot_num_change = lot_num - last_lot_num;
                        let (open_price, spread) = if lot_num_change > 0. {
                            (ask, ask - mid)
                        } else {
                            (bid, mid - bid)
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                        };
                        // update last lot num and last pos
                        last_lot_num = lot_num;
                    }

                    last_mid = mid; // update last close
                    cash.into_cast::<T>()
                })
                .collect_trusted_vec1()
        }
    }
}

#[cfg(test)]
mod tests {
    use tevec::core::testing::assert_vec1d_equal_numeric;

    use super::*;

    #[test]
    fn test_tick_future_ret_percent_signal() {
        let bid_vec = vec![101, 102, 103, 104, 103, 101, 206, 204, 208, 204, 202, 201];
        let ask_vec = vec![102, 103, 104, 105, 104, 102, 207, 205, 209, 205, 203, 202];
        let signal_vec = vec![0, 1, 1, 1, 1, 1, 1, 0, -1, -1, 1, 1];
        let contract_chg_vec = vec![
            false, false, false, false, false, true, false, false, false, false, false, false,
        ];
        let kwargs = TickFutureRetKwargs {
            init_cash: 10000,
            multiplier: 1.,
            c_rate: 0.0001,
            blowup: true,
            commission_type: CommissionType::Percent,
            signal_type: SignalType::Percent,
        };
        let res: Vec<_> = calc_tick_future_ret(
            &signal_vec,
            &bid_vec,
            &ask_vec,
            Some(&contract_chg_vec.opt()),
            &kwargs,
        );
        let expect = vec![
            10000., 10000., 10047.5009, 10144.5009, 10047.5009, 9853.5009, 9754.5318, 9660.5318,
            9636.073, 9796.1162, 9888.1162, 9791.208,
        ];
        assert_vec1d_equal_numeric(&res, &expect, Some(1e-7));
    }

    #[test]
    fn test_tick_future_ret_absolute_signal() {
        let bid_vec = vec![101, 102, 103, 104, 103, 101, 206, 204, 208, 204, 202, 201];
        let ask_vec = vec![102, 103, 104, 105, 104, 102, 207, 205, 209, 205, 203, 202];
        let signal_vec = vec![0, 1, 1, 2, 2, 1, 1, 0, -1, -1, 2, 2];
        let contract_chg_vec = vec![
            false, false, false, false, false, true, false, false, false, false, false, false,
        ];
        let kwargs = TickFutureRetKwargs {
            init_cash: 10000,
            multiplier: 1.,
            c_rate: 0.0001,
            blowup: true,
            commission_type: CommissionType::Percent,
            signal_type: SignalType::Absolute,
        };
        let res: Vec<_> = calc_tick_future_ret(
            &signal_vec,
            &bid_vec,
            &ask_vec,
            Some(&contract_chg_vec.opt()),
            &kwargs,
        );
        let expect = vec![
            10000., 10000., 10000.4897, 10001.4897, 9998.9792, 9994.9792, 9993.9589, 9991.9589,
            9991.4385, 9994.9177, 9996.9177, 9993.3568,
        ];
        assert_vec1d_equal_numeric(&res, &expect, Some(1e-7));
    }
}
