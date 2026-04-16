use serde_json::Value;
use sqlx::{Row, SqlitePool};

use crate::storage::StorageError;

// ── Records ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChatThreadRecord {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessageRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub content_json: Value,
    pub run_config_json: Option<Value>,
}

// ── Threads ───────────────────────────────────────────────────────────────────

pub async fn list_chat_threads(pool: &SqlitePool) -> Result<Vec<ChatThreadRecord>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, title, status, created_at, updated_at
         FROM chat_threads
         ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ChatThreadRecord {
            id: row.get("id"),
            title: row.get("title"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn create_chat_thread(
    pool: &SqlitePool,
    id: &str,
) -> Result<ChatThreadRecord, StorageError> {
    let mut tx = pool.begin().await?;
    let now = crate::current_utc_timestamp()?;
    sqlx::query(
        "INSERT INTO chat_threads (id, created_at, updated_at)
         VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let record = get_chat_thread_on_connection(&mut tx, id).await?;
    tx.commit().await?;
    Ok(record)
}

pub async fn get_chat_thread(
    pool: &SqlitePool,
    id: &str,
) -> Result<ChatThreadRecord, StorageError> {
    let mut connection = pool.acquire().await?;
    get_chat_thread_on_connection(&mut connection, id).await
}

async fn get_chat_thread_on_connection(
    connection: &mut sqlx::sqlite::SqliteConnection,
    id: &str,
) -> Result<ChatThreadRecord, StorageError> {
    let row = sqlx::query(
        "SELECT id, title, status, created_at, updated_at
         FROM chat_threads WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&mut *connection)
    .await?;

    Ok(ChatThreadRecord {
        id: row.get("id"),
        title: row.get("title"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn rename_chat_thread(
    pool: &SqlitePool,
    id: &str,
    title: &str,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    let now = crate::current_utc_timestamp()?;
    sqlx::query("UPDATE chat_threads SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_chat_thread_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    let now = crate::current_utc_timestamp()?;
    sqlx::query("UPDATE chat_threads SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_chat_thread(pool: &SqlitePool, id: &str) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM chat_threads WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// ── Messages ──────────────────────────────────────────────────────────────────

pub async fn list_chat_messages(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<ChatMessageRecord>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, thread_id, parent_id, content_json, run_config_json
         FROM chat_messages
         WHERE thread_id = ?
         ORDER BY created_at ASC",
    )
    .bind(thread_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let content_raw: String = row.get("content_json");
            let content_json: Value = serde_json::from_str(&content_raw)
                .map_err(|_| StorageError::Validation("invalid content_json"))?;

            let run_config_json: Option<Value> = row
                .get::<Option<String>, _>("run_config_json")
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| StorageError::Validation("invalid run_config_json"))?;

            Ok(ChatMessageRecord {
                id: row.get("id"),
                parent_id: row.get("parent_id"),
                content_json,
                run_config_json,
            })
        })
        .collect()
}

pub async fn append_chat_message(
    pool: &SqlitePool,
    thread_id: &str,
    id: &str,
    parent_id: Option<&str>,
    content_json: &Value,
    run_config_json: Option<&Value>,
) -> Result<(), StorageError> {
    let content_str = serde_json::to_string(content_json)
        .map_err(|_| StorageError::Validation("failed to serialize content_json"))?;
    let run_config_str = run_config_json
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| StorageError::Validation("failed to serialize run_config_json"))?;

    let mut tx = pool.begin().await?;

    // Upsert: update if exists (handles retries / edits)
    sqlx::query(
        "INSERT INTO chat_messages (id, thread_id, parent_id, content_json, run_config_json)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             parent_id = excluded.parent_id,
             content_json = excluded.content_json,
             run_config_json = excluded.run_config_json",
    )
    .bind(id)
    .bind(thread_id)
    .bind(parent_id)
    .bind(&content_str)
    .bind(run_config_str.as_deref())
    .execute(&mut *tx)
    .await?;

    // Touch thread updated_at so list ordering reflects activity
    let now = crate::current_utc_timestamp()?;
    sqlx::query("UPDATE chat_threads SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
