pub mod alpha_vantage;
pub mod coincap;
pub mod coingecko;
pub mod eodhd;
pub mod fcsapi;
pub mod finnhub;
pub mod fmp;
pub mod itick;
pub mod marketstack;
pub mod openfigi;
pub mod polygon;
pub mod tiingo;
pub mod twelve_data;
pub mod yahoo;

pub use alpha_vantage::{AlphaVantageProvider, fetch_alpha_vantage_quote};
pub use coincap::fetch_coincap_quote;
pub use coingecko::fetch_coingecko_quote;
pub use eodhd::{EodhdProvider, fetch_eodhd_quote};
pub use fcsapi::FcsApiProvider;
pub use finnhub::{FinnhubProvider, fetch_finnhub_quote};
pub use fmp::{FmpProvider, fetch_fmp_quote};
pub use itick::ITickProvider;
pub use marketstack::{MarketstackProvider, fetch_marketstack_quote};
pub use openfigi::fetch_openfigi_tickers;
pub use polygon::{PolygonProvider, fetch_polygon_quote};
pub use tiingo::{TiingoProvider, fetch_tiingo_quote};
pub use twelve_data::{TwelveDataProvider, fetch_twelve_data_quote};
pub use yahoo::YahooFinanceProvider;

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::Currency;

use super::{AssetPriceRefreshError, AssetQuote};

pub(super) async fn fetch_json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T, AssetPriceRefreshError> {
    let response = request.send().await.map_err(|error| {
        AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
    })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    response.json::<T>().await.map_err(|error| {
        AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
    })
}

#[async_trait::async_trait]
pub trait QuoteProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn base_url(&self) -> &str;
    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError>;
}

/// Infers the price currency from the exchange suffix in a symbol.
/// For example, `GRID.MI` → EUR (Borsa Italiana), `VOD.L` → GBP (London).
/// Falls back to USD for US symbols and unknown suffixes.
fn currency_from_symbol(symbol: &str) -> Currency {
    let suffix = match symbol.rsplit_once('.') {
        Some((_, s)) => s,
        None => return Currency::Usd,
    };
    match suffix {
        // EUR exchanges
        "MI" | "AS" | "PA" | "DE" | "F" | "BE" | "IR" | "HE" | "LS" | "VI" | "AT" => Currency::Eur,
        // GBP exchanges
        "L" | "IL" => Currency::Gbp,
        // CHF exchanges
        "SW" | "VX" => Currency::Chf,
        // USD exchanges and fallback
        _ => Currency::Usd,
    }
}

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};

fn unix_timestamp_to_rfc3339(ts: i64) -> Result<String, AssetPriceRefreshError> {
    OffsetDateTime::from_unix_timestamp(ts)
        .map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider returned invalid timestamp".into(),
            )
        })?
        .format(&Rfc3339)
        .map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: failed to format timestamp".into(),
            )
        })
}

fn normalize_provider_datetime(datetime: String) -> Result<String, AssetPriceRefreshError> {
    // Normalize ISO 8601 offsets without colon (e.g. +0000) to RFC 3339 (e.g. +00:00)
    let datetime = if datetime.len() >= 20 {
        let tail = &datetime[datetime.len() - 5..];
        let (sign, digits) = tail.split_at(1);
        if (sign == "+" || sign == "-") && digits.chars().all(|c| c.is_ascii_digit()) {
            format!(
                "{}{}{}:{}",
                &datetime[..datetime.len() - 5],
                sign,
                &digits[..2],
                &digits[2..]
            )
        } else {
            datetime
        }
    } else {
        datetime
    };

    if let Ok(value) = OffsetDateTime::parse(&datetime, &Rfc3339) {
        return value.format(&Rfc3339).map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider returned invalid datetime".into(),
            )
        });
    }

    const DATETIME_WITH_SPACE: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    const DATETIME_WITH_T: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    const DATE_ONLY: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]");

    if let Ok(value) = PrimitiveDateTime::parse(&datetime, DATETIME_WITH_SPACE) {
        return Ok(format!(
            "{}Z",
            value.format(DATETIME_WITH_T).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    if let Ok(value) = PrimitiveDateTime::parse(&datetime, DATETIME_WITH_T) {
        return Ok(format!(
            "{}Z",
            value.format(DATETIME_WITH_T).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    if let Ok(value) = Date::parse(&datetime, DATE_ONLY) {
        return Ok(format!(
            "{}T00:00:00Z",
            value.format(DATE_ONLY).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    Err(AssetPriceRefreshError::Provider(format!(
        "asset price refresh failed: unsupported datetime format: {datetime}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_from_symbol_infers_eur_for_italian_exchange() {
        assert_eq!(currency_from_symbol("GRID.MI"), Currency::Eur);
    }

    #[test]
    fn currency_from_symbol_infers_eur_for_other_european_exchanges() {
        assert_eq!(currency_from_symbol("ASML.AS"), Currency::Eur);
        assert_eq!(currency_from_symbol("AIR.PA"), Currency::Eur);
        assert_eq!(currency_from_symbol("SAP.DE"), Currency::Eur);
    }

    #[test]
    fn currency_from_symbol_infers_gbp_for_london() {
        assert_eq!(currency_from_symbol("VOD.L"), Currency::Gbp);
    }

    #[test]
    fn currency_from_symbol_infers_chf_for_swiss() {
        assert_eq!(currency_from_symbol("NESN.SW"), Currency::Chf);
        assert_eq!(currency_from_symbol("NOVN.VX"), Currency::Chf);
    }

    #[test]
    fn currency_from_symbol_defaults_to_usd_for_us_symbols() {
        assert_eq!(currency_from_symbol("AAPL"), Currency::Usd);
        assert_eq!(currency_from_symbol("AAPL.US"), Currency::Usd);
    }

    #[test]
    fn currency_from_symbol_defaults_to_usd_for_unknown_suffix() {
        assert_eq!(currency_from_symbol("STOCK.XX"), Currency::Usd);
    }
}
