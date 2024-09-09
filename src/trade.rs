use std::fmt::Debug;
use std::str::FromStr;

use derive_more::From;
use itertools::izip;
use tevec::prelude::*;

#[derive(Copy, Clone, PartialEq)]
pub enum TradeSide {
    Buy,
    Sell,
}

impl TradeSide {
    pub fn as_str(&self) -> &str {
        match self {
            TradeSide::Buy => "buy",
            TradeSide::Sell => "sell",
        }
    }
}

impl Debug for TradeSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TradeSide::Buy => "buy",
            TradeSide::Sell => "sell",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for TradeSide {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // 使用Debug的实现来格式化
        write!(f, "{:?}", self)
    }
}

impl FromStr for TradeSide {
    type Err = TError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(TradeSide::Buy),
            "sell" => Ok(TradeSide::Sell),
            _ => Err(terr!("unknown trade side: {}", s)),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Trade {
    pub time: DateTime,
    pub side: TradeSide,
    pub price: f64,
    pub num: f64,
}

impl Trade {
    #[inline]
    pub fn new(time: DateTime, side: TradeSide, price: f64, num: f64) -> Self {
        Self {
            time,
            side,
            price,
            num,
        }
    }
}

#[derive(From)]
pub enum PriceVec<I: IntoIterator> {
    // bid price vec, ask price vec
    BidAsk(I, I),
    Single(I),
}

/// Create trades info from signal,
/// note that we cann't get a real trade num,
/// num in the Trade is just a change of signal
pub fn signal_to_trades<
    V: IntoIterator<Item = T>,
    V2: IntoIterator<Item = T2>,
    VT: IntoIterator<Item = DateTime>,
    T: IsNone,
    T2: IsNone,
>(
    signal_vec: V,
    price_vec: PriceVec<V2>,
    time_vec: VT,
) -> Vec<Trade>
where
    T::Inner: Number,
    T2::Inner: Number,
{
    match price_vec {
        PriceVec::BidAsk(bid_vec, ask_vec) => {
            let mut last_signal = f64::NAN;
            let mut trades = Vec::new();
            izip!(time_vec, signal_vec, bid_vec, ask_vec).for_each(|(time, signal, bid, ask)| {
                if signal.not_none() {
                    let signal = signal.unwrap().f64();
                    if signal > last_signal {
                        trades.push(Trade::new(
                            time,
                            TradeSide::Buy,
                            ask.to_opt().map(|v| v.f64()).unwrap_or(f64::NAN),
                            signal - last_signal,
                        ));
                    } else if signal < last_signal {
                        trades.push(Trade::new(
                            time,
                            TradeSide::Sell,
                            bid.to_opt().map(|v| v.f64()).unwrap_or(f64::NAN),
                            last_signal - signal,
                        ));
                    }
                    last_signal = signal;
                }
            });
            trades
        },
        PriceVec::Single(price_vec) => {
            let mut last_signal = f64::NAN;
            let mut trades = Vec::new();
            izip!(time_vec, signal_vec, price_vec).for_each(|(time, signal, price)| {
                if signal.not_none() {
                    let signal = signal.unwrap().f64();
                    if signal > last_signal {
                        trades.push(Trade::new(
                            time,
                            TradeSide::Buy,
                            price.to_opt().map(|v| v.f64()).unwrap_or(f64::NAN),
                            signal - last_signal,
                        ));
                    } else if signal < last_signal {
                        trades.push(Trade::new(
                            time,
                            TradeSide::Sell,
                            price.to_opt().map(|v| v.f64()).unwrap_or(f64::NAN),
                            last_signal - signal,
                        ));
                    }
                    last_signal = signal;
                }
            });
            trades
        },
    }
}

#[cfg(feature = "polars")]
pub fn trade_vec_to_series(trades: &[Trade]) -> tevec::export::polars::prelude::Series {
    use tevec::export::polars::export::arrow::legacy::utils::CustomIterTools;
    use tevec::export::polars::prelude::*;
    use tevec::prelude::{IsNone, Vec1Collect};
    let len = trades.len();
    let price: Float64Chunked = trades
        .iter()
        .map(|t| t.price.to_opt())
        .collect_trusted_vec1();
    let time: DatetimeChunked = unsafe {
        trades
            .iter()
            .map(|t| t.time.into_opt_i64())
            .trust_my_length(len)
            .collect_trusted::<Int64Chunked>()
            .into_datetime(TimeUnit::Nanoseconds, None)
    };
    let num: Float64Chunked = trades.iter().map(|t| t.num.to_opt()).collect_trusted_vec1();
    let side: StringChunked = trades
        .iter()
        .map(|t| Some(t.side.as_str()))
        .collect_trusted();
    let res: StructChunked = StructChunked::from_series(
        "trade",
        &[
            time.into_series().with_name("time"),
            side.into_series().with_name("side"),
            price.into_series().with_name("price"),
            num.into_series().with_name("num"),
        ],
    )
    .unwrap();
    res.into_series()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_signal_to_trade() {
        let signal = vec![0., 0., 0.5, 0.5, 1., 0.2];
        let time = vec![
            "2021-01-01 00:00:00",
            "2021-01-01 00:01:00",
            "2021-01-01 00:02:00",
            "2021-01-01 00:03:00",
            "2021-01-01 00:04:00",
            "2021-01-01 00:05:00",
        ]
        .into_iter()
        .map(|s| DateTime::<unit::Nanosecond>::parse(s, None).unwrap())
        .collect_trusted_to_vec();
        let price = vec![10., 11., 12., 13., 14., 15.];
        let trades = signal_to_trades(
            signal.titer(),
            PriceVec::Single(price.titer()),
            time.titer(),
        );
        let expect = vec![
            Trade::new(time[2].clone(), TradeSide::Buy, price[2].clone(), 0.5),
            Trade::new(time[4].clone(), TradeSide::Buy, price[4].clone(), 0.5),
            Trade::new(time[5].clone(), TradeSide::Sell, price[5].clone(), 0.8),
        ];
        assert_eq!(trades, expect)
    }
}
