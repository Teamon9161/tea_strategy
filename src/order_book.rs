/// Represents an order book with five levels of depth.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct OrderBook {
    /// The first (best) level of the order book.
    pub level1: OrderBookLevel,
    /// The second level of the order book.
    pub level2: OrderBookLevel,
    /// The third level of the order book.
    pub level3: OrderBookLevel,
    /// The fourth level of the order book.
    pub level4: OrderBookLevel,
    /// The fifth level of the order book.
    pub level5: OrderBookLevel,
}

/// Represents a single level in the order book.
#[derive(Clone, Copy, Debug)]
pub struct OrderBookLevel {
    /// The ask (sell) price at this level.
    pub ask_price: f64,
    /// The bid (buy) price at this level.
    pub bid_price: f64,
    /// The volume available at the ask price.
    pub ask_volume: f64,
    /// The volume available at the bid price.
    pub bid_volume: f64,
}

/// Compares two f64 values for equality, considering NaN values.
#[inline]
fn f64_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    (a - b).abs() < f64::EPSILON
}

impl PartialEq for OrderBookLevel {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        f64_eq(self.ask_price, other.ask_price)
            && f64_eq(self.bid_price, other.bid_price)
            && f64_eq(self.ask_volume, other.ask_volume)
            && f64_eq(self.bid_volume, other.bid_volume)
    }
}

impl Eq for OrderBookLevel {}

impl Default for OrderBookLevel {
    #[inline]
    fn default() -> Self {
        Self {
            ask_price: f64::NAN,
            bid_price: f64::NAN,
            ask_volume: f64::NAN,
            bid_volume: f64::NAN,
        }
    }
}

impl From<(f64, f64)> for OrderBookLevel {
    #[inline]
    fn from((ask_price, bid_price): (f64, f64)) -> Self {
        Self {
            ask_price,
            bid_price,
            ..Default::default()
        }
    }
}

impl From<(f64, f64, f64, f64)> for OrderBookLevel {
    #[inline]
    fn from((ask_price, bid_price, ask_volume, bid_volume): (f64, f64, f64, f64)) -> Self {
        Self::new(ask_price, bid_price, ask_volume, bid_volume)
    }
}

impl<T> From<T> for OrderBook
where
    T: Into<OrderBookLevel>,
{
    #[inline]
    fn from(value: T) -> Self {
        Self {
            level1: value.into(),
            ..Default::default()
        }
    }
}

impl<T> From<(T, T, T, T, T)> for OrderBook
where
    T: Into<OrderBookLevel>,
{
    #[inline]
    fn from((level1, level2, level3, level4, level5): (T, T, T, T, T)) -> Self {
        Self {
            level1: level1.into(),
            level2: level2.into(),
            level3: level3.into(),
            level4: level4.into(),
            level5: level5.into(),
        }
    }
}

impl OrderBookLevel {
    /// Creates a new OrderBookLevel with the given prices and volumes.
    #[inline]
    pub fn new(ask_price: f64, bid_price: f64, ask_volume: f64, bid_volume: f64) -> Self {
        Self {
            ask_price,
            bid_price,
            ask_volume,
            bid_volume,
        }
    }

    /// Calculates the total ask amount (price * volume) for this level.
    #[inline]
    pub fn ask_amt(&self) -> f64 {
        self.ask_price * self.ask_volume
    }

    /// Calculates the total bid amount (price * volume) for this level.
    #[inline]
    pub fn bid_amt(&self) -> f64 {
        self.bid_price * self.bid_volume
    }
}

impl OrderBook {
    /// Creates a new OrderBook with the given levels.
    #[inline]
    pub fn new<T: Into<OrderBookLevel>>(
        level1: T,
        level2: T,
        level3: T,
        level4: T,
        level5: T,
    ) -> Self {
        Self {
            level1: level1.into(),
            level2: level2.into(),
            level3: level3.into(),
            level4: level4.into(),
            level5: level5.into(),
        }
    }

    /// Calculates the buy price for a given volume.
    ///
    /// If the volume is larger than the total available volume, it returns an error
    /// with the average price and available volume. Otherwise, it returns the calculated price.
    ///
    /// # Arguments
    ///
    /// * `volume` - The volume to buy
    ///
    /// # Returns
    ///
    /// * `Ok(f64)` - The calculated buy price
    /// * `Err((f64, f64))` - The average price and available volume if the requested volume exceeds the available volume
    ///
    /// # Examples
    ///
    /// ```
    /// use tea_strategy::OrderBook;
    /// let order_book = OrderBook::new(
    ///     (10., 9., 2., 1.),
    ///     (11., 8., 3., 2.),
    ///     (12., 7., 1., 1.),
    ///     (14., 6., 2., 2.),
    ///     (15., 3., 1., 1.),
    /// );
    /// assert_eq!(order_book.get_buy_price(1.), Ok(10.));
    /// assert_eq!(order_book.get_buy_price(4.), Ok(10.5));
    /// assert_eq!(order_book.get_buy_price(8.), Ok(93. / 8.));
    /// assert_eq!(order_book.get_buy_price(10.), Err((108. / 9., 9.)));
    /// ```
    pub fn get_buy_price(&self, volume: f64) -> Result<f64, (f64, f64)> {
        let mut volume = volume;
        let mut amt = 0.0;
        let mut available_volume = 0.0;
        if volume <= self.level1.ask_volume {
            return Ok(self.level1.ask_price);
        } else {
            amt += self.level1.ask_amt();
            available_volume += self.level1.ask_volume;
            volume -= self.level1.ask_volume;
        }
        if volume <= self.level2.ask_volume {
            return Ok((amt + volume * self.level2.ask_price) / (available_volume + volume));
        } else {
            amt += self.level2.ask_amt();
            available_volume += self.level2.ask_volume;
            volume -= self.level2.ask_volume;
        }
        if volume <= self.level3.ask_volume {
            return Ok((amt + volume * self.level3.ask_price) / (available_volume + volume));
        } else {
            amt += self.level3.ask_amt();
            available_volume += self.level3.ask_volume;
            volume -= self.level3.ask_volume;
        }
        if volume <= self.level4.ask_volume {
            return Ok((amt + volume * self.level4.ask_price) / (available_volume + volume));
        } else {
            amt += self.level4.ask_amt();
            available_volume += self.level4.ask_volume;
            volume -= self.level4.ask_volume;
        }
        if volume <= self.level5.ask_volume {
            return Ok((amt + volume * self.level5.ask_price) / (available_volume + volume));
        } else {
            amt += self.level5.ask_amt();
            available_volume += self.level5.ask_volume;
            // volume -= self.level5.ask_volume;
        }
        Err((amt / available_volume, available_volume))
    }

    /// Calculates the sell price for a given volume.
    ///
    /// If the volume is larger than the total available volume, it returns an error
    /// with the average price and available volume. Otherwise, it returns the calculated price.
    ///
    /// # Arguments
    ///
    /// * `volume` - The volume to sell
    ///
    /// # Returns
    ///
    /// * `Ok(f64)` - The calculated sell price
    /// * `Err((f64, f64))` - The average price and available volume if the requested volume exceeds the available volume
    ///
    /// # Examples
    ///
    /// ```
    /// use tea_strategy::OrderBook;
    /// let order_book = OrderBook::new(
    ///     (10., 9., 2., 1.),
    ///     (11., 8., 3., 2.),
    ///     (12., 7., 1., 1.),
    ///     (14., 6., 2., 2.),
    ///     (15., 3., 1., 1.),
    /// );
    /// assert_eq!(order_book.get_sell_price(2.), Ok(8.5));
    /// assert_eq!(order_book.get_sell_price(4.), Ok(8.));
    /// assert_eq!(order_book.get_sell_price(7.), Ok(47. / 7.));
    /// assert_eq!(order_book.get_sell_price(10.), Err((47. / 7., 7.)));
    /// ```
    pub fn get_sell_price(&self, volume: f64) -> Result<f64, (f64, f64)> {
        let mut volume = volume;
        let mut amt = 0.0;
        let mut available_volume = 0.0;
        if volume <= self.level1.bid_volume {
            return Ok(self.level1.bid_price);
        } else {
            amt += self.level1.bid_amt();
            available_volume += self.level1.bid_volume;
            volume -= self.level1.bid_volume;
        }
        if volume <= self.level2.bid_volume {
            return Ok((amt + volume * self.level2.bid_price) / (available_volume + volume));
        } else {
            amt += self.level2.bid_amt();
            available_volume += self.level2.bid_volume;
            volume -= self.level2.bid_volume;
        }
        if volume <= self.level3.bid_volume {
            return Ok((amt + volume * self.level3.bid_price) / (available_volume + volume));
        } else {
            amt += self.level3.bid_amt();
            available_volume += self.level3.bid_volume;
            volume -= self.level3.bid_volume;
        }
        if volume <= self.level4.bid_volume {
            return Ok((amt + volume * self.level4.bid_price) / (available_volume + volume));
        } else {
            amt += self.level4.bid_amt();
            available_volume += self.level4.bid_volume;
            volume -= self.level4.bid_volume;
        }
        if volume <= self.level5.bid_volume {
            return Ok((amt + volume * self.level5.bid_price) / (available_volume + volume));
        } else {
            amt += self.level5.bid_amt();
            available_volume += self.level5.bid_volume;
            // volume -= self.level5.bid_volume;
        }
        Err((amt / available_volume, available_volume))
    }
}
