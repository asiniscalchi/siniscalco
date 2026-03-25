pub mod alpha_vantage;
pub mod coingecko;
pub mod finnhub;
pub mod openfigi;
pub mod twelve_data;

pub use alpha_vantage::fetch_alpha_vantage_quote;
pub use coingecko::fetch_coingecko_quote;
pub use finnhub::fetch_finnhub_quote;
pub use openfigi::fetch_openfigi_tickers;
pub use twelve_data::fetch_twelve_data_quote;

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};

use super::AssetPriceRefreshError;

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
