#[cfg(feature = "pl")]
use tevec::polars::prelude::Series;
use tevec::prelude::DateTime;

#[derive(Clone)]
pub struct Trade {
    pub open_price: f64,
    pub stop_price: f64,
    pub open_time: DateTime,
    pub close_time: DateTime,
    pub num: f64,
}

impl Trade {
    #[inline]
    pub fn new(
        open_price: f64,
        stop_price: f64,
        open_time: DateTime,
        close_time: DateTime,
        num: f64,
    ) -> Self {
        Self {
            open_price,
            stop_price,
            open_time,
            close_time,
            num,
        }
    }

    #[inline]
    pub fn profit(&self, multiplier: Option<f64>) -> f64 {
        let multiplier = multiplier.unwrap_or(1.);
        (self.stop_price - self.open_price) * multiplier * self.num
    }
}

#[cfg(feature = "pl")]
pub fn trade_vec_to_series(trades: &[Trade]) -> Series {
    use tevec::polars::prelude::*;
    use tevec::polars_arrow::legacy::utils::CustomIterTools;
    use tevec::prelude::{IsNone, Vec1Collect};
    let len = trades.len();
    let open_price: Float64Chunked = trades
        .iter()
        .map(|t| t.open_price.to_opt())
        .collect_trusted_vec1();
    let stop_price: Float64Chunked = trades
        .iter()
        .map(|t| t.stop_price.to_opt())
        .collect_trusted_vec1();
    let open_time: DatetimeChunked = unsafe {
        trades
            .iter()
            .map(|t| t.open_time.into_opt_i64())
            .trust_my_length(len)
            .collect_trusted::<Int64Chunked>()
            .into_datetime(TimeUnit::Nanoseconds, None)
    };
    let close_time: DatetimeChunked = unsafe {
        trades
            .iter()
            .map(|t| t.close_time.into_opt_i64())
            .trust_my_length(len)
            .collect_trusted::<Int64Chunked>()
            .into_datetime(TimeUnit::Nanoseconds, None)
    };
    let num: Float64Chunked = trades.iter().map(|t| t.num.to_opt()).collect_trusted_vec1();
    let res: StructChunked = StructChunked::new(
        "trade",
        &[
            open_price.into_series().with_name("open_price"),
            stop_price.into_series().with_name("stop_price"),
            open_time.into_series().with_name("open_time"),
            close_time.into_series().with_name("close_time"),
            num.into_series().with_name("num"),
        ],
    )
    .unwrap();
    res.into_series()
}
