use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::ValidationError;

/// ISO 4217 currency. Only ZMW is accepted at launch (04 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Currency {
    Zmw,
}

impl Currency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zmw => "ZMW",
        }
    }
}

impl FromStr for Currency {
    type Err = ValidationError;

    /// Exactly the upper-case ISO code; no trimming or case folding.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ZMW" => Ok(Self::Zmw),
            _ => Err(ValidationError::UnsupportedCurrency),
        }
    }
}

impl TryFrom<String> for Currency {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Currency> for String {
    fn from(c: Currency) -> Self {
        c.as_str().to_owned()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An amount in integer minor units (ngwee for ZMW) with its currency.
///
/// Never floating point. Signed, because ledger postings and adjustments are
/// signed; whether an amount must be positive is the caller's domain rule.
/// JSON shape: `{ "amount_minor": 125000, "currency": "ZMW" }` (05 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Money {
    amount_minor: i64,
    currency: Currency,
}

impl Money {
    pub fn new(amount_minor: i64, currency: Currency) -> Self {
        Self {
            amount_minor,
            currency,
        }
    }

    pub fn zero(currency: Currency) -> Self {
        Self::new(0, currency)
    }

    pub fn amount_minor(self) -> i64 {
        self.amount_minor
    }

    pub fn currency(self) -> Currency {
        self.currency
    }

    pub fn is_positive(self) -> bool {
        self.amount_minor > 0
    }

    pub fn checked_add(self, other: Money) -> Result<Money, ValidationError> {
        self.combine(other, i64::checked_add)
    }

    pub fn checked_sub(self, other: Money) -> Result<Money, ValidationError> {
        self.combine(other, i64::checked_sub)
    }

    fn combine(
        self,
        other: Money,
        op: fn(i64, i64) -> Option<i64>,
    ) -> Result<Money, ValidationError> {
        if self.currency != other.currency {
            return Err(ValidationError::CurrencyMismatch);
        }
        op(self.amount_minor, other.amount_minor)
            .map(|amount| Money::new(amount, self.currency))
            .ok_or(ValidationError::AmountOutOfRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_parses_only_exact_zmw() {
        assert_eq!("ZMW".parse::<Currency>(), Ok(Currency::Zmw));
        for bad in ["zmw", " ZMW", "USD", "", "ZM"] {
            assert_eq!(
                bad.parse::<Currency>(),
                Err(ValidationError::UnsupportedCurrency),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn money_json_round_trip_uses_documented_shape() {
        let money = Money::new(125_000, Currency::Zmw);
        let json = serde_json::to_string(&money).unwrap();
        assert_eq!(json, r#"{"amount_minor":125000,"currency":"ZMW"}"#);
        assert_eq!(serde_json::from_str::<Money>(&json).unwrap(), money);
    }

    #[test]
    fn money_json_rejects_bad_input() {
        for bad in [
            r#"{"amount_minor":1.5,"currency":"ZMW"}"#,
            r#"{"amount_minor":100,"currency":"USD"}"#,
            r#"{"amount_minor":100}"#,
            r#"{"amount_minor":100,"currency":"ZMW","extra":1}"#,
            r#"{"amount_minor":"100","currency":"ZMW"}"#,
        ] {
            assert!(serde_json::from_str::<Money>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn arithmetic_checks_overflow() {
        let max = Money::new(i64::MAX, Currency::Zmw);
        let one = Money::new(1, Currency::Zmw);
        assert_eq!(max.checked_add(one), Err(ValidationError::AmountOutOfRange));
        assert_eq!(
            Money::new(i64::MIN, Currency::Zmw).checked_sub(one),
            Err(ValidationError::AmountOutOfRange)
        );
        assert_eq!(
            Money::new(85_000, Currency::Zmw).checked_sub(Money::new(30_000, Currency::Zmw)),
            Ok(Money::new(55_000, Currency::Zmw))
        );
    }

    #[test]
    fn positivity() {
        assert!(Money::new(1, Currency::Zmw).is_positive());
        assert!(!Money::zero(Currency::Zmw).is_positive());
        assert!(!Money::new(-1, Currency::Zmw).is_positive());
    }
}
