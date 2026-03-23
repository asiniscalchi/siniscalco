use std::fmt;

use rust_decimal::Decimal;

use super::storage_error::StorageError;

pub const DECIMAL_SCALE: u32 = 6;
const SCALE_FACTOR: i128 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Amount(i64);

impl Amount {
    pub fn as_decimal(self) -> Decimal {
        Decimal::new(self.0, DECIMAL_SCALE)
    }

    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    pub fn as_scaled_i64(self) -> i64 {
        self.0
    }

    pub fn from_scaled_i64(value: i64) -> Self {
        Self(value)
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
            return Err(StorageError::Validation(
                "amount must match a signed 6-decimal value",
            ));
        }

        if let Some(fractional_part) = fractional_part
            && (fractional_part.is_empty()
                || fractional_part.len() > DECIMAL_SCALE as usize
                || !fractional_part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(StorageError::Validation(
                "amount must match a signed 6-decimal value",
            ));
        }

        let integer_value = integer_part
            .parse::<i128>()
            .map_err(|_| StorageError::Validation("amount must match a signed 6-decimal value"))?;

        let fractional_value = match fractional_part {
            Some(value) => {
                let parsed = value.parse::<i128>().map_err(|_| {
                    StorageError::Validation("amount must match a signed 6-decimal value")
                })?;
                parsed * 10_i128.pow(DECIMAL_SCALE - value.len() as u32)
            }
            None => 0,
        };

        let scaled_unsigned = integer_value
            .checked_mul(SCALE_FACTOR)
            .and_then(|value| value.checked_add(fractional_value))
            .ok_or(StorageError::Validation(
                "amount must match a signed 6-decimal value",
            ))?;

        let scaled = if value.starts_with('-') {
            -scaled_unsigned
        } else {
            scaled_unsigned
        };

        if !(i64::MIN as i128..=i64::MAX as i128).contains(&scaled) {
            return Err(StorageError::Validation(
                "amount must match a signed 6-decimal value",
            ));
        }

        Ok(Self(scaled as i64))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_decimal().normalize())
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
            Amount::try_from("9223372036854.775807")
                .unwrap()
                .to_string(),
            "9223372036854.775807"
        );
    }

    #[test]
    fn rejects_invalid_amounts() {
        for value in [".", "1.", ".1", "abc", "1.1234567", "9223372036854.775808"] {
            let error = Amount::try_from(value).expect_err("invalid amount should fail");
            assert_eq!(
                error.to_string(),
                "amount must match a signed 6-decimal value"
            );
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

    #[test]
    fn round_trips_scaled_storage_values() {
        let amount = Amount::from_scaled_i64(12_345_678);

        assert_eq!(amount.as_scaled_i64(), 12_345_678);
        assert_eq!(amount.to_string(), "12.345678");
    }
}
