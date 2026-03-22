use std::fmt;

use super::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AccountId(i64);

impl AccountId {
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for AccountId {
    type Error = StorageError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(StorageError::Validation(
                "account_id must be greater than zero",
            ));
        }

        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::AccountId;

    #[test]
    fn accepts_positive_ids() {
        assert_eq!(AccountId::try_from(1).unwrap().as_i64(), 1);
        assert_eq!(AccountId::try_from(42).unwrap().as_i64(), 42);
    }

    #[test]
    fn rejects_non_positive_ids() {
        for value in [0, -1] {
            let error = AccountId::try_from(value).expect_err("invalid account id should fail");
            assert_eq!(error.to_string(), "account_id must be greater than zero");
        }
    }
}
