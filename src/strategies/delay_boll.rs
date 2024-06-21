use crate::StrategyFilter;
use itertools::izip;
use serde::Deserialize;
use tevec::prelude::*;

#[derive(Deserialize)]
pub struct DelayBollKwargs {
    // window, open_width, stop_width, delay_width
    pub params: (usize, f64, f64, f64, Option<f64>),
    pub min_periods: Option<usize>,
    pub long_signal: f64,
    pub short_signal: f64,
    pub close_signal: f64,
}

macro_rules! boll_logic_impl {
    (
        $kwargs: expr,
        $fac: expr, $middle: expr, $std: expr,
        $last_signal: expr, $last_fac: expr, $delay_open_flag: expr,
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
                if !$delay_open_flag {
                    let long_delay_cond = ($last_signal <= $kwargs.close_signal) && (fac >= $kwargs.params.1) $(&& $long_open.unwrap_or(true))? $(&& $long_open_cond)?;
                    let short_delay_cond = ($last_signal >= $kwargs.close_signal) && (fac <= -$kwargs.params.1) $(&& $short_open.unwrap_or(true))? $(&& $short_open_cond)?;
                    if long_delay_cond || short_delay_cond {
                        $delay_open_flag = true;
                    }
                } else if $delay_open_flag {
                    if ($last_fac > $kwargs.params.3) && (fac <= $kwargs.params.3) $(&& $long_open.unwrap_or(true))? {
                        $delay_open_flag = false;
                        $last_signal = $kwargs.long_signal;
                    } else if ($last_fac < -$kwargs.params.3) && (fac >= -$kwargs.params.3) $(&& $short_open.unwrap_or(true))? {
                        $delay_open_flag = false;
                        $last_signal = $kwargs.short_signal;
                    }
                }

                if let Some(chase_bound) = $kwargs.params.4 {
                    if ($last_fac < chase_bound) && (fac >= chase_bound) $(&& $long_open.unwrap_or(true))? {
                        $last_signal = $kwargs.long_signal;
                        $delay_open_flag = false;
                    } else if ($last_fac > -chase_bound) && (fac <= -chase_bound) $(&& $short_open.unwrap_or(true))? {
                        $last_signal = $kwargs.short_signal;
                        $delay_open_flag = false;
                    }
                }

                // == stop condition
                if $last_signal != $kwargs.close_signal {
                    // we can skip stop condition if trade is already close or open
                    if (($last_fac > $kwargs.params.2) && (fac <= $kwargs.params.2))
                        $(|| $long_stop.unwrap_or(false))?  // additional stop condition
                    {
                        // long stop
                        $last_signal = $kwargs.close_signal;
                        $delay_open_flag = false;
                    } else if (($last_fac < -$kwargs.params.2) && (fac >= -$kwargs.params.2))
                        $(|| $short_stop.unwrap_or(false))?
                    {
                        // short stop
                        $last_signal = $kwargs.close_signal;
                        $delay_open_flag = false;
                    }
                }
                // == update open info
                $last_fac = fac;
            }
            $last_signal
        }
    };
}

#[allow(clippy::collapsible_else_if)]
pub fn delay_boll<O: Vec1<T::Cast<f64>>, T, V: Vec1View<T>, VMask: Vec1View<Option<bool>>>(
    fac_arr: &V,
    filter: Option<&StrategyFilter<VMask>>,
    kwargs: &DelayBollKwargs,
) -> TResult<O>
where
    T: IsNone,
    T::Inner: Number,
{
    tensure!(
        (kwargs.params.3 > kwargs.params.2) && (kwargs.params.3 <= kwargs.params.1),
        "delay_width should be greater than stop_width and less than open_width"
    );
    if let Some(chase_param) = kwargs.params.4 {
        tensure!(
            kwargs.params.1 < chase_param,
            "open_width should be less than chase_param"
        )
    }
    // let m = kwargs.params.1;
    let mut last_signal = kwargs.close_signal;
    let mut last_fac = 0.;
    let mut delay_open_flag = false;
    let min_periods = kwargs.min_periods.unwrap_or(kwargs.params.0 / 2);

    let middle_arr: Vec<f64> = fac_arr.ts_vmean(kwargs.params.0, Some(min_periods));
    let std_arr: Vec<f64> = fac_arr.ts_vstd(kwargs.params.0, Some(min_periods));

    let out = if let Some(filter) = filter {
        let zip_ = izip!(
            fac_arr.titer(),
            middle_arr.titer(),
            std_arr.titer(),
            filter.titer(),
        );
        zip_.map(
            |(fac, middle, std, (long_open, long_stop, short_open, short_stop))| {
                T::inner_cast(boll_logic_impl!(
                    kwargs, fac, middle, std,
                    last_signal, last_fac, delay_open_flag,
                    filters=>(long_open, long_stop, short_open, short_stop),
                ))
            },
        )
        .collect_trusted_vec1()
    } else {
        let zip_ = izip!(fac_arr.titer(), middle_arr.titer(), std_arr.titer(),);
        zip_.map(|(fac, middle, std)| {
            T::inner_cast(boll_logic_impl!(
                kwargs,
                fac,
                middle,
                std,
                last_signal,
                last_fac,
                delay_open_flag,
            ))
        })
        .collect_trusted_vec1()
    };
    Ok(out)
}
