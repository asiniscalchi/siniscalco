use sqlx::{Row, SqlitePool};

use crate::storage::records::*;
use crate::storage::{StorageError, TradeDate, current_utc_timestamp};

pub async fn create_todo(
    pool: &SqlitePool,
    input: CreateTodoInput,
) -> Result<TodoRecord, StorageError> {
    let title = normalize_title(input.title)?;
    let symbol = normalize_symbol(input.symbol)?;
    let timestamp = current_utc_timestamp()?;

    let result = sqlx::query(
        r#"
        INSERT INTO todos (title, due_date, symbol, completed, created_at, updated_at)
        VALUES (?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(title)
    .bind(input.due_date.as_str())
    .bind(symbol)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(pool)
    .await?;

    get_todo(pool, result.last_insert_rowid()).await
}

pub async fn list_todos(pool: &SqlitePool) -> Result<Vec<TodoRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, title, due_date, symbol, completed, created_at, updated_at
        FROM todos
        ORDER BY completed ASC, due_date ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_todo_row).collect()
}

pub async fn update_todo_completed(
    pool: &SqlitePool,
    id: i64,
    completed: bool,
) -> Result<TodoRecord, StorageError> {
    let timestamp = current_utc_timestamp()?;
    let result = sqlx::query(
        r#"
        UPDATE todos
        SET completed = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(completed)
    .bind(&timestamp)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    get_todo(pool, id).await
}

pub async fn delete_todo(pool: &SqlitePool, id: i64) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

async fn get_todo(pool: &SqlitePool, id: i64) -> Result<TodoRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, title, due_date, symbol, completed, created_at, updated_at
        FROM todos
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    map_todo_row(row)
}

fn normalize_title(title: String) -> Result<String, StorageError> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(StorageError::Validation("todo title must not be empty"));
    }
    Ok(title)
}

fn normalize_symbol(symbol: Option<String>) -> Result<Option<String>, StorageError> {
    symbol
        .map(|s| {
            let normalized = s.trim().to_uppercase();
            if normalized.is_empty() {
                Err(StorageError::Validation("todo symbol must not be empty"))
            } else {
                Ok(normalized)
            }
        })
        .transpose()
}

fn map_todo_row(row: sqlx::sqlite::SqliteRow) -> Result<TodoRecord, StorageError> {
    Ok(TodoRecord {
        id: row.get("id"),
        title: row.get("title"),
        due_date: TradeDate::try_from(row.get::<&str, _>("due_date"))?,
        symbol: row.get("symbol"),
        completed: row.get::<i64, _>("completed") == 1,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
