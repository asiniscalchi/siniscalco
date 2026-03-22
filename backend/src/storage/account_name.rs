use std::fmt;

use super::models::StorageError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountName(String);

impl AccountName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AccountName {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(StorageError::Validation("name must not be empty"));
        }

        Ok(Self(value.to_string()))
    }
}

impl fmt::Display for AccountName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::AccountName;

    #[test]
    fn accepts_non_empty_names() {
        assert_eq!(AccountName::try_from("IBKR").unwrap().as_str(), "IBKR");
        assert_eq!(AccountName::try_from("Main Bank").unwrap().as_str(), "Main Bank");
    }

    #[test]
    fn rejects_empty_names() {
        for value in ["", " ", "   "] {
            let error = AccountName::try_from(value).expect_err("empty name should fail");
            assert_eq!(error.to_string(), "name must not be empty");
        }
    }
}
