use std::time::Duration;

use sqlx::SqlitePool;
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    AccountSummaryStatus, PRODUCT_BASE_CURRENCY, current_utc_timestamp_iso8601,
    get_portfolio_summary, insert_portfolio_snapshot_if_missing,
};

/// Hour (UTC) at which the daily portfolio snapshot is taken.
/// 22:00 UTC is after US market close for most trading days.
const SNAPSHOT_HOUR_UTC: u8 = 22;

pub async fn spawn_portfolio_snapshot_task(pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            sleep(duration_until_next_snapshot()).await;
            record_portfolio_snapshot(&pool).await;
        }
    });
}

fn duration_until_next_snapshot() -> Duration {
    let now = OffsetDateTime::now_utc();
    let today_target =
        now.replace_time(time::Time::from_hms(SNAPSHOT_HOUR_UTC, 0, 0).expect("valid time"));

    let next_target = if now < today_target {
        today_target
    } else {
        today_target + time::Duration::days(1)
    };

    let secs = (next_target - now).whole_seconds().max(0) as u64;
    Duration::from_secs(secs)
}

async fn record_portfolio_snapshot(pool: &SqlitePool) {
    let summary = match get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY).await {
        Ok(s) => s,
        Err(error) => {
            warn!(error = %error, "portfolio snapshot: failed to compute summary");
            return;
        }
    };

    if summary.total_value_status != AccountSummaryStatus::Ok {
        return;
    }

    let Some(total_value) = summary.total_value_amount else {
        return;
    };

    let recorded_at = match current_utc_timestamp_iso8601() {
        Ok(ts) => ts,
        Err(error) => {
            warn!(error = %error, "portfolio snapshot: failed to get timestamp");
            return;
        }
    };

    if let Err(error) =
        insert_portfolio_snapshot_if_missing(pool, total_value, PRODUCT_BASE_CURRENCY, &recorded_at)
            .await
    {
        warn!(error = %error, "portfolio snapshot: failed to insert");
    } else {
        info!("portfolio snapshot recorded");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_until_next_snapshot_is_at_most_24_hours() {
        let d = duration_until_next_snapshot();
        assert!(d <= Duration::from_secs(24 * 3600));
    }

    #[test]
    fn duration_until_next_snapshot_is_positive() {
        let d = duration_until_next_snapshot();
        assert!(d > Duration::ZERO);
    }
}
