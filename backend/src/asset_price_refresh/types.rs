use crate::{AssetUnitPrice, Currency};

#[derive(Debug, Eq, PartialEq)]
pub struct AssetQuote {
    pub price: AssetUnitPrice,
    pub currency: Currency,
    pub as_of: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetPriceRefreshError {
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    Storage(#[from] crate::storage::StorageError),
}
