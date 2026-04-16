use crate::storage::records::*;
use crate::storage::{AccountId, AccountName, AccountType, Currency, StorageError};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnection};

pub async fn create_account(
    pool: &SqlitePool,
    input: CreateAccountInput,
) -> Result<AccountId, StorageError> {
    let mut tx = pool.begin().await?;

    let result =
        sqlx::query("INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)")
            .bind(input.name.as_str())
            .bind(input.account_type.as_str())
            .bind(input.base_currency.as_str())
            .execute(&mut *tx)
            .await?;

    let id = AccountId::try_from(result.last_insert_rowid())?;
    tx.commit().await?;
    Ok(id)
}

pub async fn list_accounts(pool: &SqlitePool) -> Result<Vec<AccountRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AccountRecord {
                id: AccountId::try_from(row.get::<i64, _>("id"))?,
                name: AccountName::try_from(row.get::<&str, _>("name"))?,
                account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
                base_currency: Currency::try_from(row.get::<&str, _>("base_currency"))?,
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

pub async fn get_account(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<AccountRecord, StorageError> {
    let mut connection = pool.acquire().await?;
    get_account_on_connection(&mut connection, account_id).await
}

async fn get_account_on_connection(
    connection: &mut SqliteConnection,
    account_id: AccountId,
) -> Result<AccountRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        WHERE id = ?
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_one(&mut *connection)
    .await?;

    Ok(AccountRecord {
        id: AccountId::try_from(row.get::<i64, _>("id"))?,
        name: AccountName::try_from(row.get::<&str, _>("name"))?,
        account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
        base_currency: Currency::try_from(row.get::<&str, _>("base_currency"))?,
        created_at: row.get("created_at"),
    })
}

pub async fn update_account(
    pool: &SqlitePool,
    account_id: AccountId,
    input: CreateAccountInput,
) -> Result<AccountRecord, StorageError> {
    let mut tx = pool.begin().await?;

    let existing_account = get_account_on_connection(&mut tx, account_id).await?;

    if input.base_currency != existing_account.base_currency {
        return Err(StorageError::Validation(
            "base_currency cannot be changed after account creation",
        ));
    }

    let result = sqlx::query(
        r#"
        UPDATE accounts
        SET name = ?, account_type = ?
        WHERE id = ?
        "#,
    )
    .bind(input.name.as_str())
    .bind(input.account_type.as_str())
    .bind(account_id.as_i64())
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    let record = get_account_on_connection(&mut tx, account_id).await?;
    tx.commit().await?;
    Ok(record)
}

pub async fn delete_account(pool: &SqlitePool, account_id: AccountId) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;

    let has_entries =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cash_entries WHERE account_id = ?")
            .bind(account_id.as_i64())
            .fetch_one(&mut *tx)
            .await?;

    if has_entries > 0 {
        return Err(StorageError::Validation(
            "cannot delete an account that has ledger entries",
        ));
    }

    let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id.as_i64())
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    tx.commit().await?;
    Ok(())
}
