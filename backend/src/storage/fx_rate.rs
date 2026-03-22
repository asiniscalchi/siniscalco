use std::fmt;

use rust_decimal::Decimal;

use super::{amount::Amount, storage_error::StorageError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FxRate(Amount);

impl FxRate {
    pub fn as_decimal(self) -> Decimal {
        self.0.as_decimal()
    }
}

impl TryFrom<&str> for FxRate {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let amount = Amount::try_from(value)?;
        Self::try_from(amount)
    }
}

impl TryFrom<Amount> for FxRate {
    type Error = StorageError;

    fn try_from(value: Amount) -> Result<Self, Self::Error> {
        if !value.is_positive() {
            return Err(StorageError::Validation("rate must be greater than zero"));
        }

        Ok(Self(value))
    }
}

impl fmt::Display for FxRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::FxRate;
    use crate::Amount;

    #[test]
    fn parses_positive_rates() {
        assert_eq!(FxRate::try_from("0.92").unwrap().to_string(), "0.92");
        assert_eq!(
            FxRate::try_from("1.00000000").unwrap().to_string(),
            "1.00000000"
        );
    }

    #[test]
    fn rejects_non_positive_rates() {
        for value in ["0", "0.00000000", "-0.1"] {
            let error = FxRate::try_from(value).expect_err("non-positive rate should fail");
            assert_eq!(error.to_string(), "rate must be greater than zero");
        }
    }

    #[test]
    fn rejects_invalid_rate_format() {
        let error = FxRate::try_from("1.123456789").expect_err("invalid rate should fail");
        assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
    }

    #[test]
    fn converts_positive_amounts() {
        let rate = FxRate::try_from(Amount::try_from("1.25").unwrap()).unwrap();
        assert_eq!(rate.to_string(), "1.25");
    }
}
