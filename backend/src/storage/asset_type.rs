use super::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetType {
    Stock,
    Etf,
    Bond,
    Crypto,
    CashEquivalent,
    Other,
}

impl AssetType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stock => "STOCK",
            Self::Etf => "ETF",
            Self::Bond => "BOND",
            Self::Crypto => "CRYPTO",
            Self::CashEquivalent => "CASH_EQUIVALENT",
            Self::Other => "OTHER",
        }
    }
}

impl TryFrom<&str> for AssetType {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "STOCK" => Ok(Self::Stock),
            "ETF" => Ok(Self::Etf),
            "BOND" => Ok(Self::Bond),
            "CRYPTO" => Ok(Self::Crypto),
            "CASH_EQUIVALENT" => Ok(Self::CashEquivalent),
            "OTHER" => Ok(Self::Other),
            _ => Err(StorageError::Validation(
                "asset_type must be one of: STOCK, ETF, BOND, CRYPTO, CASH_EQUIVALENT, OTHER",
            )),
        }
    }
}
