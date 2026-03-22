use serde::{Deserialize, Serialize};

use super::models::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Currency {
    Eur,
    Usd,
    Gbp,
    Chf,
}

impl Currency {
    pub const fn all() -> [Self; 4] {
        [Self::Chf, Self::Eur, Self::Gbp, Self::Usd]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eur => "EUR",
            Self::Usd => "USD",
            Self::Gbp => "GBP",
            Self::Chf => "CHF",
        }
    }
}

impl TryFrom<&str> for Currency {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "EUR" => Ok(Self::Eur),
            "USD" => Ok(Self::Usd),
            "GBP" => Ok(Self::Gbp),
            "CHF" => Ok(Self::Chf),
            _ => Err(StorageError::Validation(
                "currency must be one of: EUR, USD, GBP, CHF",
            )),
        }
    }
}

impl Serialize for Currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value.as_str()).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Currency;

    #[test]
    fn parses_supported_currency_codes() {
        assert_eq!(Currency::try_from("CHF").unwrap(), Currency::Chf);
        assert_eq!(Currency::try_from("EUR").unwrap(), Currency::Eur);
        assert_eq!(Currency::try_from("GBP").unwrap(), Currency::Gbp);
        assert_eq!(Currency::try_from("USD").unwrap(), Currency::Usd);
    }

    #[test]
    fn rejects_invalid_currency_codes() {
        let error = Currency::try_from("usd").expect_err("unsupported currency should fail");

        assert_eq!(
            error.to_string(),
            "currency must be one of: EUR, USD, GBP, CHF"
        );
    }
}
