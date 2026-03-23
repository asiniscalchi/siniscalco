use std::fmt;

use super::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AssetId(i64);

impl AssetId {
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for AssetId {
    type Error = StorageError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(StorageError::Validation("asset_id must be greater than zero"));
        }

        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::AssetId;

    #[test]
    fn accepts_positive_ids() {
        assert_eq!(AssetId::try_from(1).unwrap().as_i64(), 1);
    }

    #[test]
    fn rejects_non_positive_ids() {
        let error = AssetId::try_from(0).expect_err("invalid asset id should fail");
        assert_eq!(error.to_string(), "asset_id must be greater than zero");
    }
}
