use sqlx::{Row, SqlitePool};

use crate::storage::records::*;
use crate::storage::{
    AssetId, AssetName, AssetQuantity, AssetSymbol, AssetType, AssetUnitPrice, Currency,
    StorageError, current_utc_timestamp_iso8601,
};

pub async fn create_asset(
    pool: &SqlitePool,
    input: CreateAssetInput,
) -> Result<AssetId, StorageError> {
    let timestamp = current_utc_timestamp_iso8601()?;
    let result = sqlx::query(
        r#"
        INSERT INTO assets (symbol, name, asset_type, quote_symbol, isin, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(input.symbol.as_str())
    .bind(input.name.as_str())
    .bind(input.asset_type.as_str())
    .bind(input.quote_symbol.as_deref())
    .bind(input.isin.as_deref())
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(pool)
    .await?;

    AssetId::try_from(result.last_insert_rowid())
}

pub async fn list_assets(pool: &SqlitePool) -> Result<Vec<AssetRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            assets.id,
            assets.symbol,
            assets.name,
            assets.asset_type,
            assets.quote_symbol,
            assets.isin,
            asset_prices.price,
            asset_prices.currency_code,
            asset_prices.as_of,
            (
                SELECT SUM(CASE transaction_type WHEN 'BUY' THEN quantity ELSE -quantity END)
                FROM asset_transactions
                WHERE asset_id = assets.id
            ) as total_quantity,
            assets.created_at,
            assets.updated_at
        FROM assets
        LEFT JOIN asset_prices ON asset_prices.asset_id = assets.id
        ORDER BY symbol, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_asset_row).collect()
}

pub async fn get_asset(pool: &SqlitePool, asset_id: AssetId) -> Result<AssetRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT
            assets.id,
            assets.symbol,
            assets.name,
            assets.asset_type,
            assets.quote_symbol,
            assets.isin,
            asset_prices.price,
            asset_prices.currency_code,
            asset_prices.as_of,
            (
                SELECT SUM(CASE transaction_type WHEN 'BUY' THEN quantity ELSE -quantity END)
                FROM asset_transactions
                WHERE asset_id = assets.id
            ) as total_quantity,
            assets.created_at,
            assets.updated_at
        FROM assets
        LEFT JOIN asset_prices ON asset_prices.asset_id = assets.id
        WHERE assets.id = ?
        "#,
    )
    .bind(asset_id.as_i64())
    .fetch_one(pool)
    .await?;

    map_asset_row(row)
}

pub async fn update_asset(
    pool: &SqlitePool,
    asset_id: AssetId,
    input: UpdateAssetInput,
) -> Result<AssetRecord, StorageError> {
    let timestamp = current_utc_timestamp_iso8601()?;
    let result = sqlx::query(
        r#"
        UPDATE assets
        SET symbol = ?, name = ?, asset_type = ?, quote_symbol = ?, isin = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(input.symbol.as_str())
    .bind(input.name.as_str())
    .bind(input.asset_type.as_str())
    .bind(input.quote_symbol.as_deref())
    .bind(input.isin.as_deref())
    .bind(&timestamp)
    .bind(asset_id.as_i64())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    get_asset(pool, asset_id).await
}

pub async fn delete_asset(pool: &SqlitePool, asset_id: AssetId) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM assets WHERE id = ?")
        .bind(asset_id.as_i64())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

fn map_asset_row(row: sqlx::sqlite::SqliteRow) -> Result<AssetRecord, StorageError> {
    let total_quantity = row
        .get::<Option<i64>, _>("total_quantity")
        .filter(|&q| q > 0)
        .map(AssetQuantity::from_scaled_i64)
        .transpose()?;

    Ok(AssetRecord {
        id: AssetId::try_from(row.get::<i64, _>("id"))?,
        symbol: AssetSymbol::try_from(row.get::<&str, _>("symbol"))?,
        name: AssetName::try_from(row.get::<&str, _>("name"))?,
        asset_type: AssetType::try_from(row.get::<&str, _>("asset_type"))?,
        quote_symbol: row.get::<Option<String>, _>("quote_symbol"),
        isin: row.get::<Option<String>, _>("isin"),
        current_price: row
            .get::<Option<i64>, _>("price")
            .map(AssetUnitPrice::from_scaled_i64)
            .transpose()?,
        current_price_currency: row
            .get::<Option<String>, _>("currency_code")
            .map(|currency| Currency::try_from(currency.as_str()))
            .transpose()?,
        current_price_as_of: row.get::<Option<String>, _>("as_of"),
        total_quantity,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
