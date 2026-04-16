use super::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetTransactionType {
    Buy,
    Sell,
    Opening,
}

impl AssetTransactionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::Opening => "OPENING",
        }
    }

    pub fn has_cash_impact(self) -> bool {
        matches!(self, Self::Buy | Self::Sell)
    }
}

impl TryFrom<&str> for AssetTransactionType {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "BUY" => Ok(Self::Buy),
            "SELL" => Ok(Self::Sell),
            "OPENING" => Ok(Self::Opening),
            _ => Err(StorageError::Validation(
                "transaction_type must be one of: BUY, SELL, OPENING",
            )),
        }
    }
}
