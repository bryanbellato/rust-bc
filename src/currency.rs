use std::fmt;

pub const SATOSHIS_PER_COIN: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(u64);

#[derive(Debug)]
pub enum AmountError {
    Overflow,
    NonPositive,
    ParseError(String),
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmountError::Overflow => write!(f, "amount overflow"),
            AmountError::NonPositive => write!(f, "amount must be positive"),
            AmountError::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for AmountError {}

impl Amount {
    // create coin from satoshis
    pub fn from_satoshis(satoshis: u64) -> Self {
        Amount(satoshis)
    }

    /* create from coins
     for instance: 1.5 coins = 150,000,000 satoshis
    */
    pub fn from_coins(coins: f64) -> Result<Self, AmountError> {
        if coins < 0.0 {
            return Err(AmountError::NonPositive);
        }

        let satoshis = (coins * SATOSHIS_PER_COIN as f64).round() as u64;

        if satoshis == 0 && coins > 0.0 {
            return Err(AmountError::ParseError(
                "amount too small (minimum 1 satoshi)".to_string(),
            ));
        }

        Ok(Amount(satoshis))
    }

    // getter
    pub fn satoshis(&self) -> u64 {
        self.0
    }

    /* convert to coins for display
    for instance: 150000000 satoshis = 1.5 coins */
    pub fn as_coins(&self) -> f64 {
        self.0 as f64 / SATOSHIS_PER_COIN as f64
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    // add two amounts
    pub fn checked_add(&self, other: Amount) -> Result<Amount, AmountError> {
        self.0
            .checked_add(other.0)
            .map(Amount)
            .ok_or(AmountError::Overflow)
    }

    // subtract two amounts
    pub fn checked_sub(&self, other: Amount) -> Result<Amount, AmountError> {
        self.0
            .checked_sub(other.0)
            .map(Amount)
            .ok_or(AmountError::NonPositive)
    }
}

/* display as coins
 for instance: "1.50000000 coins"
*/
impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.8}", self.as_coins())
    }
}
