use std::fmt;

use rust_decimal::Decimal;

use super::{amount::Amount, storage_error::StorageError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetQuantity(Amount);

impl AssetQuantity {
    pub fn as_decimal(self) -> Decimal {
        self.0.as_decimal()
    }

    pub fn as_scaled_i64(self) -> i64 {
        self.0.as_scaled_i64()
    }

    pub fn from_scaled_i64(value: i64) -> Result<Self, StorageError> {
        Self::try_from(Amount::from_scaled_i64(value))
    }
}

impl TryFrom<&str> for AssetQuantity {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let amount = Amount::try_from(value)?;
        Self::try_from(amount)
    }
}

impl TryFrom<Amount> for AssetQuantity {
    type Error = StorageError;

    fn try_from(value: Amount) -> Result<Self, Self::Error> {
        if !value.is_positive() {
            return Err(StorageError::Validation(
                "quantity must be greater than zero",
            ));
        }

        Ok(Self(value))
    }
}

impl fmt::Display for AssetQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
