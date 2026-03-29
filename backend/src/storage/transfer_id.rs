use crate::storage::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferId(i64);

impl TransferId {
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for TransferId {
    type Error = StorageError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(StorageError::Validation("transfer id must be positive"));
        }
        Ok(Self(value))
    }
}
