use std::fmt;

use rust_decimal::Decimal;

use super::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Amount(Decimal);

impl Amount {
    pub fn as_decimal(self) -> Decimal {
        self.0
    }

    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }
}

impl TryFrom<&str> for Amount {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.strip_prefix('-').unwrap_or(value);

        if normalized.is_empty() {
            return Err(StorageError::Validation("amount must not be empty"));
        }

        let (integer_part, fractional_part) = match normalized.split_once('.') {
            Some((integer_part, fractional_part)) => (integer_part, Some(fractional_part)),
            None => (normalized, None),
        };

        if integer_part.is_empty() || !integer_part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
        }

        if let Some(fractional_part) = fractional_part
            && (fractional_part.is_empty()
                || fractional_part.len() > 8
                || !fractional_part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
        }

        let total_digits = integer_part.len() + fractional_part.map_or(0, str::len);
        if total_digits > 20 || integer_part.len() > 12 {
            return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
        }

        let decimal = value
            .parse::<Decimal>()
            .map_err(|_| StorageError::Validation("amount must match DECIMAL(20,8)"))?;

        Ok(Self(decimal))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Amount;

    #[test]
    fn parses_valid_amounts() {
        assert_eq!(Amount::try_from("12").unwrap().to_string(), "12");
        assert_eq!(Amount::try_from("12.3").unwrap().to_string(), "12.3");
        assert_eq!(Amount::try_from("-12.3").unwrap().to_string(), "-12.3");
        assert_eq!(
            Amount::try_from("999999999999.12345678")
                .unwrap()
                .to_string(),
            "999999999999.12345678"
        );
    }

    #[test]
    fn rejects_invalid_amounts() {
        for value in [".", "1.", ".1", "abc", "1.123456789", "1234567890123"] {
            let error = Amount::try_from(value).expect_err("invalid amount should fail");
            assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
        }
    }

    #[test]
    fn rejects_empty_amounts() {
        let error = Amount::try_from("").expect_err("empty amount should fail");

        assert_eq!(error.to_string(), "amount must not be empty");
    }

    #[test]
    fn reports_positivity() {
        assert!(Amount::try_from("0.1").unwrap().is_positive());
        assert!(!Amount::try_from("0").unwrap().is_positive());
        assert!(!Amount::try_from("-0.1").unwrap().is_positive());
    }
}
