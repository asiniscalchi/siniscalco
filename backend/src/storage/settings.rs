use sqlx::SqlitePool;

use super::StorageError;

pub async fn get_app_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, StorageError> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(value)
}

pub async fn set_app_setting(
    pool: &SqlitePool,
    key: &str,
    value: &str,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_app_setting(pool: &SqlitePool, key: &str) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM app_settings WHERE key = ?")
        .bind(key)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
