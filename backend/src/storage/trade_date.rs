use std::fmt;

use time::{Date, Month};

use super::storage_error::StorageError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeDate(String);

impl TradeDate {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for TradeDate {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 10 || &value[4..5] != "-" || &value[7..8] != "-" {
            return Err(StorageError::Validation("trade_date must use YYYY-MM-DD"));
        }

        let year = value[0..4]
            .parse::<i32>()
            .map_err(|_| StorageError::Validation("trade_date must use YYYY-MM-DD"))?;
        let month = value[5..7]
            .parse::<u8>()
            .map_err(|_| StorageError::Validation("trade_date must use YYYY-MM-DD"))?;
        let day = value[8..10]
            .parse::<u8>()
            .map_err(|_| StorageError::Validation("trade_date must use YYYY-MM-DD"))?;

        let month =
            Month::try_from(month).map_err(|_| StorageError::Validation("trade_date must use YYYY-MM-DD"))?;
        Date::from_calendar_date(year, month, day)
            .map_err(|_| StorageError::Validation("trade_date must use YYYY-MM-DD"))?;

        Ok(Self(value.to_string()))
    }
}

impl fmt::Display for TradeDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
