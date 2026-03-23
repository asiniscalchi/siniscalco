use std::fmt;

use rust_decimal::Decimal;

use super::{amount::Amount, storage_error::StorageError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetPosition(Amount);

impl AssetPosition {
    pub fn as_decimal(self) -> Decimal {
        self.0.as_decimal()
    }
}

impl TryFrom<Decimal> for AssetPosition {
    type Error = StorageError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value <= Decimal::ZERO {
            return Err(StorageError::Validation(
                "position quantity must be greater than zero",
            ));
        }

        Ok(Self(Amount::try_from(value.to_string().as_str())?))
    }
}

impl fmt::Display for AssetPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
