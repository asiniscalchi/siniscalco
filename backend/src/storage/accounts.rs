use crate::storage::{AccountId, AccountName, Currency};
use crate::storage::models::*;
use sqlx::{Row, SqlitePool};

pub async fn create_account(
    pool: &SqlitePool,
    input: CreateAccountInput,
) -> Result<AccountId, StorageError> {
    let result =
        sqlx::query("INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)")
            .bind(input.name.as_str())
            .bind(input.account_type.as_str())
            .bind(input.base_currency.as_str())
            .execute(pool)
            .await?;

    AccountId::try_from(result.last_insert_rowid())
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
                id: AccountId::try_from(row.get::<i64, _>("id"))
                    .expect("stored account id should be valid"),
                name: AccountName::try_from(row.get::<&str, _>("name"))
                    .expect("stored account name should be valid"),
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
    let row = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        WHERE id = ?
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_one(pool)
    .await?;

    Ok(AccountRecord {
        id: AccountId::try_from(row.get::<i64, _>("id")).expect("stored account id should be valid"),
        name: AccountName::try_from(row.get::<&str, _>("name"))
            .expect("stored account name should be valid"),
        account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
        base_currency: Currency::try_from(row.get::<&str, _>("base_currency"))?,
        created_at: row.get("created_at"),
    })
}

pub async fn delete_account(pool: &SqlitePool, account_id: AccountId) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id.as_i64())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}
