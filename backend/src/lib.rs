use std::{error::Error, fmt, fs, path::Path, str::FromStr};

use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

const UTC_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountType {
    Bank,
    Broker,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bank => "bank",
            Self::Broker => "broker",
        }
    }
}

impl TryFrom<&str> for AccountType {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "bank" => Ok(Self::Bank),
            "broker" => Ok(Self::Broker),
            _ => Err(StorageError::Validation(
                "account_type must be one of: bank, broker",
            )),
        }
    }
}

#[derive(Debug)]
pub enum StorageError {
    Validation(&'static str),
    Database(sqlx::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StorageError {}

impl From<sqlx::Error> for StorageError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub struct CreateAccountInput<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
    pub base_currency: &'a str,
}

pub struct UpsertAccountBalanceInput<'a> {
    pub account_id: i64,
    pub currency: &'a str,
    pub amount: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountRecord {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub created_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountBalanceRecord {
    pub account_id: i64,
    pub currency: String,
    pub amount: String,
    pub updated_at: String,
}

/// SQLite stores DECIMAL values with numeric affinity, so it does not preserve
/// input formatting like trailing zeroes. We keep DECIMAL(20,8) in the schema
/// for clarity and validate writes in the application, but read values back via
/// `CAST(amount AS TEXT)` and treat exact display formatting as an application concern.
pub async fn connect_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    init_db(&pool).await?;
    Ok(pool)
}

pub async fn connect_db_file(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| sqlx::Error::Configuration(Box::new(error)))?;
        }
    }

    let url = format!("sqlite://{}", path.as_ref().display());
    connect_db(&url).await
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

pub async fn create_account(
    pool: &SqlitePool,
    input: CreateAccountInput<'_>,
) -> Result<i64, StorageError> {
    validate_name(input.name)?;
    validate_currency(input.base_currency)?;

    let result =
        sqlx::query("INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)")
            .bind(input.name)
            .bind(input.account_type.as_str())
            .bind(input.base_currency)
            .execute(pool)
            .await?;

    Ok(result.last_insert_rowid())
}

pub async fn upsert_account_balance(
    pool: &SqlitePool,
    input: UpsertAccountBalanceInput<'_>,
) -> Result<(), StorageError> {
    validate_currency(input.currency)?;
    validate_decimal_20_8(input.amount)?;

    let updated_at = current_utc_timestamp()?;

    sqlx::query(
        r#"
        INSERT INTO account_balances (account_id, currency, amount, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(account_id, currency) DO UPDATE SET
            amount = excluded.amount,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.account_id)
    .bind(input.currency)
    .bind(input.amount)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
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
                id: row.get("id"),
                name: row.get("name"),
                account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
                base_currency: row.get("base_currency"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

pub async fn list_account_balances(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            account_id,
            currency,
            CAST(amount AS TEXT) AS amount,
            updated_at
        FROM account_balances
        WHERE account_id = ?
        ORDER BY currency
        "#,
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AccountBalanceRecord {
            account_id: row.get("account_id"),
            currency: row.get("currency"),
            amount: row.get("amount"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

fn validate_name(name: &str) -> Result<(), StorageError> {
    if name.trim().is_empty() {
        return Err(StorageError::Validation("name must not be empty"));
    }

    Ok(())
}

fn validate_currency(currency: &str) -> Result<(), StorageError> {
    let is_valid = currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase());

    if !is_valid {
        return Err(StorageError::Validation(
            "currency must be a 3-letter uppercase code",
        ));
    }

    Ok(())
}

fn validate_decimal_20_8(amount: &str) -> Result<(), StorageError> {
    let amount = amount.strip_prefix('-').unwrap_or(amount);

    if amount.is_empty() {
        return Err(StorageError::Validation("amount must not be empty"));
    }

    let (integer_part, fractional_part) = match amount.split_once('.') {
        Some((integer_part, fractional_part)) => (integer_part, Some(fractional_part)),
        None => (amount, None),
    };

    if integer_part.is_empty() || !integer_part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    if let Some(fractional_part) = fractional_part {
        if fractional_part.is_empty()
            || fractional_part.len() > 8
            || !fractional_part.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
        }
    }

    let total_digits = integer_part.len() + fractional_part.map_or(0, str::len);
    if total_digits > 20 || integer_part.len() > 12 {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    Ok(())
}

fn current_utc_timestamp() -> Result<String, StorageError> {
    OffsetDateTime::now_utc()
        .format(UTC_TIMESTAMP_FORMAT)
        .map_err(|_| StorageError::Validation("failed to generate UTC timestamp"))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::NamedTempFile;

    use super::{
        AccountBalanceRecord, AccountRecord, AccountType, CreateAccountInput, StorageError,
        UpsertAccountBalanceInput, connect_db_file, create_account, init_db, list_account_balances,
        list_accounts, upsert_account_balance,
    };

    async fn test_pool() -> sqlx::SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("in-memory sqlite connect options should parse")
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory sqlite pool should connect");

        init_db(&pool).await.expect("schema should initialize");
        pool
    }

    #[tokio::test]
    async fn applies_migrations_and_creates_tables() {
        let pool = test_pool().await;

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', '_sqlx_migrations')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 3);
    }

    #[tokio::test]
    async fn bootstraps_file_database_and_runs_migrations() {
        let file = NamedTempFile::new().expect("temp db file should be created");
        let pool = connect_db_file(file.path())
            .await
            .expect("file database should initialize");

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 2);
    }

    #[tokio::test]
    async fn creates_account_without_balance() {
        let pool = test_pool().await;

        create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .expect("account count query should succeed");

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn reads_accounts_in_insert_order() {
        let pool = test_pool().await;

        create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("first account insert should succeed");

        create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("second account insert should succeed");

        let accounts = list_accounts(&pool)
            .await
            .expect("account list should succeed");

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].name, "Main Bank");
        assert_eq!(accounts[0].account_type, AccountType::Bank);
        assert_eq!(accounts[1].name, "IBKR");
        assert_eq!(accounts[1].account_type, AccountType::Broker);
    }

    #[tokio::test]
    async fn allows_multiple_currencies_per_account() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        for (currency, amount) in [("EUR", "12000.00000000"), ("USD", "3500.00000000")] {
            upsert_account_balance(
                &pool,
                UpsertAccountBalanceInput {
                    account_id,
                    currency,
                    amount,
                },
            )
            .await
            .expect("balance insert should succeed");
        }

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert_eq!(
            balances,
            vec![
                AccountBalanceRecord {
                    account_id,
                    currency: "EUR".to_string(),
                    amount: "12000".to_string(),
                    updated_at: balances[0].updated_at.clone(),
                },
                AccountBalanceRecord {
                    account_id,
                    currency: "USD".to_string(),
                    amount: "3500".to_string(),
                    updated_at: balances[1].updated_at.clone(),
                }
            ]
        );
        assert_eq!(balances[0].updated_at.len(), 19);
        assert_eq!(balances[1].updated_at.len(), 19);
    }

    #[tokio::test]
    async fn upsert_updates_existing_balance() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "10.00000000",
            },
        )
        .await
        .expect("first balance insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.00000000",
            },
        )
        .await
        .expect("upsert should update the existing balance");

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].amount, "12");
        assert_eq!(balances[0].updated_at.len(), 19);
    }

    #[tokio::test]
    async fn rejects_invalid_account_type_input() {
        let error =
            AccountType::try_from("cash").expect_err("unsupported account type should fail");

        assert_eq!(
            error.to_string(),
            "account_type must be one of: bank, broker"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_account_currency_input() {
        let pool = test_pool().await;

        let error = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "usd",
            },
        )
        .await
        .expect_err("lowercase currency should fail");

        assert_eq!(
            error.to_string(),
            "currency must be a 3-letter uppercase code"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_balance_currency_input() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("account insert should succeed");

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "us",
                amount: "10.00000000",
            },
        )
        .await
        .expect_err("invalid currency should fail");

        assert_eq!(
            error.to_string(),
            "currency must be a 3-letter uppercase code"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_amount_input() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "EUR",
                amount: "1.123456789",
            },
        )
        .await
        .expect_err("amount with more than 8 decimals should fail");

        assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
    }

    #[tokio::test]
    async fn rejects_balance_for_missing_account() {
        let pool = test_pool().await;

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id: 999_i64,
                currency: "USD",
                amount: "10.00000000",
            },
        )
        .await
        .expect_err("missing parent account should fail");

        match error {
            StorageError::Database(error) => {
                assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
            }
            StorageError::Validation(_) => panic!("expected database error"),
        }
    }

    #[tokio::test]
    async fn preserves_created_account_fields() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Joint Bank",
                account_type: AccountType::Bank,
                base_currency: "GBP",
            },
        )
        .await
        .expect("account insert should succeed");

        let accounts = list_accounts(&pool)
            .await
            .expect("account list should succeed");

        assert_eq!(
            accounts,
            vec![AccountRecord {
                id: account_id,
                name: "Joint Bank".to_string(),
                account_type: AccountType::Bank,
                base_currency: "GBP".to_string(),
                created_at: accounts[0].created_at.clone(),
            }]
        );
        assert_eq!(accounts[0].created_at.len(), 19);
    }

    #[tokio::test]
    async fn migration_metadata_contains_the_initial_migration() {
        let pool = test_pool().await;

        let version: i64 = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migration metadata query should succeed");

        assert_eq!(version, 1);
    }
}
