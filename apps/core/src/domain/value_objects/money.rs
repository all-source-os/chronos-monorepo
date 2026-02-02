use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// Supported currencies in the paywall system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// US Dollar (for display purposes)
    USD,
    /// USDC stablecoin on Solana
    USDC,
    /// Native SOL token
    SOL,
}

impl Currency {
    /// Get the number of decimal places for this currency
    pub fn decimals(&self) -> u8 {
        match self {
            Currency::USD => 2,
            Currency::USDC => 6, // USDC has 6 decimals on Solana
            Currency::SOL => 9,  // SOL has 9 decimals
        }
    }

    /// Get the currency symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::USD => "$",
            Currency::USDC => "USDC",
            Currency::SOL => "SOL",
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Value Object: Money
///
/// Represents a monetary amount with currency in the paywall system.
/// Amounts are stored as the smallest unit (e.g., cents for USD, lamports for SOL).
///
/// Domain Rules:
/// - Amount must be non-negative
/// - Currency must be specified
/// - Immutable once created
/// - Operations preserve currency (cannot add USD to SOL)
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Amount in smallest unit (cents, lamports, etc.)
    amount: u64,
    /// The currency
    currency: Currency,
}

impl Money {
    /// Create a new Money value from the smallest unit
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::{Money, Currency};
    ///
    /// // 50 cents
    /// let money = Money::new(50, Currency::USD);
    /// assert_eq!(money.amount(), 50);
    /// assert_eq!(money.currency(), Currency::USD);
    /// ```
    pub fn new(amount: u64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Create a Money value from a decimal amount
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::{Money, Currency};
    ///
    /// // $0.50
    /// let money = Money::from_decimal(0.50, Currency::USD);
    /// assert_eq!(money.amount(), 50);
    /// ```
    pub fn from_decimal(decimal: f64, currency: Currency) -> Self {
        let multiplier = 10_u64.pow(currency.decimals() as u32);
        let amount = (decimal * multiplier as f64).round() as u64;
        Self { amount, currency }
    }

    /// Create zero Money
    pub fn zero(currency: Currency) -> Self {
        Self {
            amount: 0,
            currency,
        }
    }

    /// Create USD Money from cents
    pub fn usd_cents(cents: u64) -> Self {
        Self::new(cents, Currency::USD)
    }

    /// Create USD Money from dollars (decimal)
    pub fn usd(dollars: f64) -> Self {
        Self::from_decimal(dollars, Currency::USD)
    }

    /// Create USDC Money from smallest unit
    pub fn usdc(amount: u64) -> Self {
        Self::new(amount, Currency::USDC)
    }

    /// Create USDC Money from decimal
    pub fn usdc_decimal(amount: f64) -> Self {
        Self::from_decimal(amount, Currency::USDC)
    }

    /// Create SOL Money from lamports
    pub fn lamports(lamports: u64) -> Self {
        Self::new(lamports, Currency::SOL)
    }

    /// Create SOL Money from decimal
    pub fn sol(amount: f64) -> Self {
        Self::from_decimal(amount, Currency::SOL)
    }

    /// Get the amount in smallest unit
    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Get the currency
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Get the amount as a decimal
    pub fn as_decimal(&self) -> f64 {
        let divisor = 10_u64.pow(self.currency.decimals() as u32);
        self.amount as f64 / divisor as f64
    }

    /// Check if this amount is zero
    pub fn is_zero(&self) -> bool {
        self.amount == 0
    }

    /// Check if this amount is positive (non-zero)
    pub fn is_positive(&self) -> bool {
        self.amount > 0
    }

    /// Check if this amount is at least a minimum value
    pub fn at_least(&self, minimum: &Money) -> Result<()> {
        if self.currency != minimum.currency {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Cannot compare {} with {}",
                self.currency, minimum.currency
            )));
        }

        if self.amount < minimum.amount {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Amount {} is less than minimum {}",
                self.as_decimal(),
                minimum.as_decimal()
            )));
        }

        Ok(())
    }

    /// Calculate a percentage of this amount
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::{Money, Currency};
    ///
    /// let money = Money::usd_cents(1000); // $10.00
    /// let fee = money.percentage(7); // 7% = 70 cents
    /// assert_eq!(fee.amount(), 70);
    /// ```
    pub fn percentage(&self, percent: u64) -> Self {
        let amount = (self.amount * percent) / 100;
        Self {
            amount,
            currency: self.currency,
        }
    }

    /// Subtract a percentage and return the remainder
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::{Money, Currency};
    ///
    /// let money = Money::usd_cents(1000); // $10.00
    /// let after_fee = money.subtract_percentage(7); // 7% fee = $9.30
    /// assert_eq!(after_fee.amount(), 930);
    /// ```
    pub fn subtract_percentage(&self, percent: u64) -> Self {
        let fee = (self.amount * percent) / 100;
        Self {
            amount: self.amount.saturating_sub(fee),
            currency: self.currency,
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.currency {
            Currency::USD => write!(f, "${:.2}", self.as_decimal()),
            Currency::USDC => write!(f, "{:.2} USDC", self.as_decimal()),
            Currency::SOL => write!(f, "{:.4} SOL", self.as_decimal()),
        }
    }
}

impl Add for Money {
    type Output = Result<Money>;

    fn add(self, other: Money) -> Self::Output {
        if self.currency != other.currency {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Cannot add {} to {}",
                self.currency, other.currency
            )));
        }

        Ok(Money {
            amount: self.amount + other.amount,
            currency: self.currency,
        })
    }
}

impl Sub for Money {
    type Output = Result<Money>;

    fn sub(self, other: Money) -> Self::Output {
        if self.currency != other.currency {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Cannot subtract {} from {}",
                other.currency, self.currency
            )));
        }

        Ok(Money {
            amount: self.amount.saturating_sub(other.amount),
            currency: self.currency,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_money_from_smallest_unit() {
        let money = Money::new(50, Currency::USD);
        assert_eq!(money.amount(), 50);
        assert_eq!(money.currency(), Currency::USD);
    }

    #[test]
    fn test_create_money_from_decimal() {
        let money = Money::from_decimal(0.50, Currency::USD);
        assert_eq!(money.amount(), 50);
        assert_eq!(money.currency(), Currency::USD);

        let money = Money::from_decimal(1.00, Currency::USDC);
        assert_eq!(money.amount(), 1_000_000); // 6 decimals
    }

    #[test]
    fn test_usd_helpers() {
        let cents = Money::usd_cents(150);
        assert_eq!(cents.amount(), 150);
        assert_eq!(cents.as_decimal(), 1.50);

        let dollars = Money::usd(1.50);
        assert_eq!(dollars.amount(), 150);
    }

    #[test]
    fn test_usdc_helpers() {
        let usdc = Money::usdc_decimal(1.0);
        assert_eq!(usdc.amount(), 1_000_000);
        assert_eq!(usdc.currency(), Currency::USDC);
    }

    #[test]
    fn test_sol_helpers() {
        let sol = Money::sol(1.0);
        assert_eq!(sol.amount(), 1_000_000_000);
        assert_eq!(sol.currency(), Currency::SOL);

        let lamports = Money::lamports(1_000_000_000);
        assert_eq!(lamports.as_decimal(), 1.0);
    }

    #[test]
    fn test_zero() {
        let zero = Money::zero(Currency::USD);
        assert!(zero.is_zero());
        assert!(!zero.is_positive());
    }

    #[test]
    fn test_is_positive() {
        let money = Money::usd_cents(100);
        assert!(money.is_positive());
        assert!(!money.is_zero());
    }

    #[test]
    fn test_at_least() {
        let amount = Money::usd_cents(100);
        let minimum = Money::usd_cents(50);

        assert!(amount.at_least(&minimum).is_ok());

        let small = Money::usd_cents(25);
        assert!(small.at_least(&minimum).is_err());
    }

    #[test]
    fn test_at_least_different_currency_fails() {
        let usd = Money::usd_cents(100);
        let sol = Money::lamports(100);

        assert!(usd.at_least(&sol).is_err());
    }

    #[test]
    fn test_percentage() {
        let money = Money::usd_cents(1000); // $10.00
        let fee = money.percentage(7); // 7%
        assert_eq!(fee.amount(), 70); // 70 cents
    }

    #[test]
    fn test_subtract_percentage() {
        let money = Money::usd_cents(1000); // $10.00
        let after_fee = money.subtract_percentage(7); // 7% fee
        assert_eq!(after_fee.amount(), 930); // $9.30
    }

    #[test]
    fn test_add_same_currency() {
        let a = Money::usd_cents(100);
        let b = Money::usd_cents(50);
        let result = (a + b).unwrap();
        assert_eq!(result.amount(), 150);
    }

    #[test]
    fn test_add_different_currency_fails() {
        let usd = Money::usd_cents(100);
        let sol = Money::lamports(100);
        let result = usd + sol;
        assert!(result.is_err());
    }

    #[test]
    fn test_sub_same_currency() {
        let a = Money::usd_cents(100);
        let b = Money::usd_cents(50);
        let result = (a - b).unwrap();
        assert_eq!(result.amount(), 50);
    }

    #[test]
    fn test_sub_saturating() {
        let a = Money::usd_cents(50);
        let b = Money::usd_cents(100);
        let result = (a - b).unwrap();
        assert_eq!(result.amount(), 0); // Saturates at 0
    }

    #[test]
    fn test_sub_different_currency_fails() {
        let usd = Money::usd_cents(100);
        let sol = Money::lamports(100);
        let result = usd - sol;
        assert!(result.is_err());
    }

    #[test]
    fn test_display_usd() {
        let money = Money::usd_cents(150);
        assert_eq!(format!("{}", money), "$1.50");
    }

    #[test]
    fn test_display_usdc() {
        let money = Money::usdc_decimal(1.50);
        assert_eq!(format!("{}", money), "1.50 USDC");
    }

    #[test]
    fn test_display_sol() {
        let money = Money::sol(0.001);
        assert_eq!(format!("{}", money), "0.0010 SOL");
    }

    #[test]
    fn test_currency_decimals() {
        assert_eq!(Currency::USD.decimals(), 2);
        assert_eq!(Currency::USDC.decimals(), 6);
        assert_eq!(Currency::SOL.decimals(), 9);
    }

    #[test]
    fn test_currency_symbol() {
        assert_eq!(Currency::USD.symbol(), "$");
        assert_eq!(Currency::USDC.symbol(), "USDC");
        assert_eq!(Currency::SOL.symbol(), "SOL");
    }

    #[test]
    fn test_equality() {
        let a = Money::usd_cents(100);
        let b = Money::usd_cents(100);
        let c = Money::usd_cents(200);
        let d = Money::new(100, Currency::SOL);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d); // Different currency
    }

    #[test]
    fn test_cloning() {
        let a = Money::usd_cents(100);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let a = Money::usd_cents(100);
        let b = Money::usd_cents(100);

        let mut set = HashSet::new();
        set.insert(a);

        assert!(set.contains(&b));
    }

    #[test]
    fn test_serde_serialization() {
        let money = Money::usd_cents(150);

        // Serialize
        let json = serde_json::to_string(&money).unwrap();

        // Deserialize
        let deserialized: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, money);
    }

    #[test]
    fn test_as_decimal_precision() {
        // Test USD (2 decimals)
        let usd = Money::usd_cents(123);
        assert!((usd.as_decimal() - 1.23).abs() < f64::EPSILON);

        // Test USDC (6 decimals)
        let usdc = Money::usdc(1_234_567);
        assert!((usdc.as_decimal() - 1.234567).abs() < 0.000001);

        // Test SOL (9 decimals)
        let sol = Money::lamports(1_234_567_890);
        assert!((sol.as_decimal() - 1.23456789).abs() < 0.000000001);
    }
}
