use std::str::FromStr;

use itertools::izip;
use serde::{Deserialize, Deserializer};
use tevec::prelude::*;

use super::{CommissionType, SignalType};

#[derive(Deserialize)]
pub struct TickFutureRetFullKwargs {
    pub init_cash: usize,
    pub multiplier: f64,
    pub c_rate: f64,
    pub blowup: bool,
    pub commission_type: CommissionType,
    pub signal_type: SignalType,
    pub open_price_method: OpenPriceMethod,
}

impl Default for TickFutureRetFullKwargs {
    #[inline]
    fn default() -> Self {
        TickFutureRetFullKwargs {
            init_cash: 0,
            multiplier: 1.,
            c_rate: 0.0003,
            blowup: false,
            commission_type: CommissionType::Percent,
            signal_type: SignalType::Absolute,
            open_price_method: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Profit {
    pub unrealize: f64,
    pub realize: f64,
    pub open_price: f64,
}

impl From<(f64, f64, f64)> for Profit {
    #[inline]
    fn from((unrealize, realize, open_price): (f64, f64, f64)) -> Self {
        Profit {
            unrealize,
            realize,
            open_price,
        }
    }
}

#[derive(Default)]
pub enum OpenPriceMethod {
    #[default]
    Average,
    First,
    Last,
}

impl FromStr for OpenPriceMethod {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "first" => Ok(OpenPriceMethod::First),
            "average" => Ok(OpenPriceMethod::Average),
            "last" => Ok(OpenPriceMethod::Last),
            _ => Err("invalid open price method".to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for OpenPriceMethod {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

pub fn calc_tick_future_ret_full<T, V, VMask>(
    signal_vec: &V,
    bid_vec: &V,
    ask_vec: &V,
    contract_chg_signal_vec: Option<&VMask>,
    kwargs: &TickFutureRetFullKwargs,
) -> Vec<Profit>
where
    T: IsNone,
    T::Inner: Number,
    V: Vec1View<T>,
    VMask: Vec1View<Option<bool>>,
{
    if signal_vec.is_empty() {
        return Vec::empty();
    }
    let init_cash = kwargs.init_cash as f64;
    let mut cash = init_cash;
    let mut last_lot_num = 0.;
    let mut last_chg = false;
    let mut last_mid = f64::NAN;
    let mut average_open_price = f64::NAN;
    let mut realize_profit = 0.;
    let blowup = kwargs.blowup;
    let multiplier = kwargs.multiplier;
    let commission_type = kwargs.commission_type;
    let c_rate = kwargs.c_rate;
    if let SignalType::Absolute = kwargs.signal_type {
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
                    return (cash, realize_profit, average_open_price).into();
                } else if blowup && cash < 0. {
                    return (0., realize_profit, average_open_price).into();
                }
                let lot_num = lot_num.unwrap().f64();
                let bid = bid.unwrap().f64();
                let ask = ask.unwrap().f64();
                let chg = chg.unwrap_or(false);
                let mid = (bid + ask) * 0.5;

                if last_chg && last_lot_num != 0. {
                    average_open_price = if last_lot_num > 0. { ask } else { bid };
                    if let CommissionType::Percent = commission_type {
                        cash -=
                            last_lot_num.abs() * multiplier * (mid * c_rate + (ask - bid) * 0.5);
                        realize_profit -=
                            last_lot_num.abs() * average_open_price * c_rate * multiplier;
                    } else {
                        cash -= last_lot_num.abs() * (c_rate + (ask - bid) * 0.5 * multiplier);
                        realize_profit -= last_lot_num.abs() * c_rate;
                    }
                }
                // calculate the profit and loss of the current period
                // we should not calculate the profit in this way if the contract has changed
                if (last_lot_num != 0.) && last_mid.not_none() && (!last_chg) {
                    cash += last_lot_num * (mid - last_mid) * multiplier;
                }
                let out = (cash - init_cash, realize_profit, average_open_price).into();
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
                        if last_lot_num == 0. {
                            average_open_price = open_price;
                        } else if lot_num == 0. {
                            realize_profit +=
                                (open_price - average_open_price) * last_lot_num * multiplier;
                            average_open_price = f64::NAN;
                        } else if lot_num.signum() != last_lot_num.signum() {
                            realize_profit +=
                                last_lot_num * (open_price - average_open_price) * multiplier;
                            average_open_price = open_price;
                        } else if last_lot_num.abs() > lot_num.abs() {
                            realize_profit += (open_price - average_open_price)
                                * (last_lot_num.abs() - lot_num.abs())
                                * multiplier;
                        } else if last_lot_num.abs() < lot_num.abs() {
                            average_open_price = match kwargs.open_price_method {
                                OpenPriceMethod::First => average_open_price,
                                OpenPriceMethod::Last => open_price,
                                OpenPriceMethod::Average => {
                                    (average_open_price * last_lot_num.abs()
                                        + open_price * lot_num_change.abs())
                                        / lot_num.abs()
                                },
                            }
                        } else {
                            panic!("implemention error");
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                            realize_profit -=
                                lot_num_change.abs() * open_price * c_rate * multiplier;
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                            realize_profit -= lot_num_change.abs() * c_rate;
                        }
                    } else {
                        let open_price = if last_lot_num > 0. { bid } else { ask };
                        realize_profit +=
                            (open_price - average_open_price) * last_lot_num * multiplier;
                        average_open_price = f64::NAN;
                        // for simple, assume spread is (bid - ask) / 2
                        // otherwise we need the bid and ask of next hot future
                        if let CommissionType::Percent = commission_type {
                            cash -= last_lot_num.abs()
                                * multiplier
                                * (mid * c_rate + (ask - bid) * 0.5);
                            realize_profit -= last_lot_num.abs() * open_price * c_rate * multiplier;
                        } else {
                            cash -= last_lot_num.abs() * (c_rate + (ask - bid) * 0.5 * multiplier);
                            realize_profit -= last_lot_num.abs() * c_rate;
                        }
                    };

                    // update last lot num and last pos
                    last_lot_num = lot_num;
                }

                last_mid = mid; // update last close
                last_chg = chg;
                out
            })
            .collect_trusted_vec1()
        } else {
            // ignore contract chg signal
            // this should be faster than the above
            izip!(signal_vec.titer(), bid_vec.titer(), ask_vec.titer(),)
                .map(|(lot_num, bid, ask)| {
                    if lot_num.is_none() || bid.is_none() || ask.is_none() {
                        return (cash, realize_profit, average_open_price).into();
                    } else if blowup && cash < 0. {
                        return (0., realize_profit, average_open_price).into();
                    }
                    let lot_num = lot_num.unwrap().f64();
                    let bid = bid.unwrap().f64();
                    let ask = ask.unwrap().f64();
                    let mid = (bid + ask) * 0.5;

                    // calculate the profit and loss of the current period
                    if (last_lot_num != 0.) && last_mid.not_none() {
                        cash += last_lot_num * (mid - last_mid) * multiplier;
                    }
                    let out = (cash - init_cash, realize_profit, average_open_price).into();
                    // addup the commision fee
                    if lot_num != last_lot_num {
                        let lot_num_change = lot_num - last_lot_num;
                        let (open_price, spread) = if lot_num_change > 0. {
                            (ask, ask - mid)
                        } else {
                            (bid, mid - bid)
                        };
                        if last_lot_num == 0. {
                            average_open_price = open_price;
                        } else if lot_num == 0. {
                            realize_profit +=
                                (open_price - average_open_price) * last_lot_num * multiplier;
                            average_open_price = f64::NAN;
                        } else if lot_num.signum() != last_lot_num.signum() {
                            realize_profit +=
                                last_lot_num * (open_price - average_open_price) * multiplier;
                            average_open_price = open_price;
                        } else if last_lot_num.abs() > lot_num.abs() {
                            realize_profit += (open_price - average_open_price)
                                * (last_lot_num.abs() - lot_num.abs())
                                * multiplier;
                        } else if last_lot_num.abs() < lot_num.abs() {
                            average_open_price = match kwargs.open_price_method {
                                OpenPriceMethod::First => average_open_price,
                                OpenPriceMethod::Last => open_price,
                                OpenPriceMethod::Average => {
                                    (average_open_price * last_lot_num.abs()
                                        + open_price * lot_num_change.abs())
                                        / lot_num.abs()
                                },
                            }
                        } else {
                            panic!("implemention error");
                        };
                        if let CommissionType::Percent = commission_type {
                            cash -=
                                lot_num_change.abs() * multiplier * (open_price * c_rate + spread);
                            realize_profit -=
                                lot_num_change.abs() * open_price * c_rate * multiplier;
                        } else {
                            cash -= lot_num_change.abs() * (c_rate + spread * multiplier);
                            realize_profit -= lot_num_change.abs() * c_rate;
                        }
                        // update last lot num and last pos
                        last_lot_num = lot_num;
                    }

                    last_mid = mid; // update last close
                    out
                })
                .collect_trusted_vec1()
        }
    } else {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use tevec::core::testing::assert_vec1d_equal_numeric;

    use super::*;

    #[test]
    fn test_tick_future_ret_full_absolute_signal() {
        let bid_vec = vec![101, 102, 103, 104, 103, 101, 206, 204, 208, 204, 202, 201];
        let ask_vec = vec![102, 103, 104, 105, 104, 102, 207, 205, 209, 205, 203, 202];
        let signal_vec = vec![0, 1, 1, 2, 2, 1, 1, 0, -1, -1, 2, 2];
        let contract_chg_vec = vec![
            false, false, false, false, false, true, false, false, false, false, false, false,
        ];
        let kwargs = TickFutureRetFullKwargs {
            c_rate: 0.0001,
            ..Default::default()
        };
        let res: Vec<_> = calc_tick_future_ret_full(
            &signal_vec,
            &bid_vec,
            &ask_vec,
            Some(&contract_chg_vec.opt()),
            &kwargs,
        );

        let expect_open_price = vec![
            f64::NAN,
            f64::NAN,
            103.,
            103.,
            104.,
            104.,
            207.,
            207.,
            f64::NAN,
            208.,
            208.,
            203.,
        ];
        assert_vec1d_equal_numeric(
            &res.iter().map(|p| p.open_price).collect::<Vec<_>>(),
            &expect_open_price,
            Some(1e-7),
        );
        let expect_realize_profit = vec![
            0., 0., -0.0103, -0.0103, -0.0208, -0.0208, -6.0617, -6.0617, -9.0821, -9.1029,
            -9.1029, -4.1638,
        ];
        assert_vec1d_equal_numeric(
            &res.iter().map(|p| p.realize).collect::<Vec<_>>(),
            &expect_realize_profit,
            Some(1e-7),
        );
        let expect_unrealize_profit = vec![
            10000.0, 10000.0, 10000.4897, 10001.4897, 9998.9792, 9994.9792, 9993.43825, 9991.43825,
            9990.91785, 9994.39705, 9996.39705, 9992.83615,
        ]
        .into_iter()
        .map(|v| v - 10000.);
        assert_vec1d_equal_numeric(
            &res.iter().map(|p| p.unrealize).collect::<Vec<_>>(),
            &expect_unrealize_profit.collect::<Vec<_>>(),
            Some(1e-7),
        );
    }
}

#[cfg(feature = "polars")]
pub fn profit_vec_to_series(trades: &[Profit]) -> tevec::polars::prelude::Series {
    use tevec::polars::prelude::*;
    use tevec::prelude::{IsNone, Vec1Collect};
    let unrealized_profit: Float64Chunked = trades
        .iter()
        .map(|t| t.unrealize.to_opt())
        .collect_trusted_vec1();
    let realized_profit: Float64Chunked = trades
        .iter()
        .map(|t| t.realize.to_opt())
        .collect_trusted_vec1();
    let open_price: Float64Chunked = trades
        .iter()
        .map(|t| t.open_price.to_opt())
        .collect_trusted_vec1();
    let res: StructChunked = StructChunked::new(
        "profit",
        &[
            unrealized_profit
                .into_series()
                .with_name("unrealized_profit"),
            realized_profit.into_series().with_name("realized_profit"),
            open_price.into_series().with_name("open_price"),
        ],
    )
    .unwrap();
    res.into_series()
}
