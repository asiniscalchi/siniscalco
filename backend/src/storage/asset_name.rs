use std::fmt;

use super::storage_error::StorageError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetName(String);

impl AssetName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AssetName {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(StorageError::Validation("asset name must not be empty"));
        }

        Ok(Self(value.to_string()))
    }
}

impl fmt::Display for AssetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
