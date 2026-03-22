use std::{error::Error, fmt};

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

const UTC_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

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
    Internal(&'static str),
    Database(sqlx::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Internal(message) => f.write_str(message),
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

pub struct UpsertFxRateInput<'a> {
    pub from_currency: &'a str,
    pub to_currency: &'a str,
    pub rate: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UpsertOutcome {
    Created,
    Updated,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountRecord {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSummaryStatus {
    Ok,
    ConversionUnavailable,
}

impl AccountSummaryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ConversionUnavailable => "conversion_unavailable",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountSummaryRecord {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub summary_status: AccountSummaryStatus,
    pub total_amount: Option<String>,
    pub total_currency: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountBalanceRecord {
    pub account_id: i64,
    pub currency: String,
    pub amount: String,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurrencyRecord {
    pub code: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateRecord {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: String,
}

pub async fn create_account(
    pool: &SqlitePool,
    input: CreateAccountInput<'_>,
) -> Result<i64, StorageError> {
    validate_name(input.name)?;
    validate_allowed_currency(pool, input.base_currency).await?;

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
) -> Result<UpsertOutcome, StorageError> {
    validate_allowed_currency(pool, input.currency).await?;
    validate_decimal_20_8(input.amount)?;

    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM account_balances WHERE account_id = ? AND currency = ?)",
    )
    .bind(input.account_id)
    .bind(input.currency)
    .fetch_one(&mut *transaction)
    .await?
        != 0;

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
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}

pub async fn upsert_fx_rate(
    pool: &SqlitePool,
    input: UpsertFxRateInput<'_>,
) -> Result<UpsertOutcome, StorageError> {
    validate_allowed_currency(pool, input.from_currency).await?;
    validate_allowed_currency(pool, input.to_currency).await?;
    validate_decimal_20_8(input.rate)?;
    validate_positive_decimal(input.rate)?;

    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM fx_rates WHERE from_currency = ? AND to_currency = ?)",
    )
    .bind(input.from_currency)
    .bind(input.to_currency)
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO fx_rates (from_currency, to_currency, rate)
        VALUES (?, ?, ?)
        ON CONFLICT(from_currency, to_currency) DO UPDATE SET
            rate = excluded.rate
        "#,
    )
    .bind(input.from_currency)
    .bind(input.to_currency)
    .bind(input.rate)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
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

pub async fn list_account_summaries(
    pool: &SqlitePool,
) -> Result<Vec<AccountSummaryRecord>, StorageError> {
    let accounts = list_accounts(pool).await?;
    let mut summaries = Vec::with_capacity(accounts.len());

    for account in accounts {
        let balances = list_account_balances(pool, account.id).await?;
        let summary = summarize_account(pool, &account, &balances).await?;

        summaries.push(AccountSummaryRecord {
            id: account.id,
            name: account.name,
            account_type: account.account_type,
            base_currency: account.base_currency,
            summary_status: summary.status,
            total_amount: summary.total_amount,
            total_currency: summary.total_currency,
        });
    }

    Ok(summaries)
}

pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<CurrencyRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT code
        FROM currencies
        ORDER BY code
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CurrencyRecord {
            code: row.get("code"),
        })
        .collect())
}

pub async fn list_fx_rates(pool: &SqlitePool) -> Result<Vec<FxRateRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            from_currency,
            to_currency,
            CAST(rate AS TEXT) AS rate
        FROM fx_rates
        ORDER BY from_currency, to_currency
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FxRateRecord {
            from_currency: row.get("from_currency"),
            to_currency: row.get("to_currency"),
            rate: row.get("rate"),
        })
        .collect())
}

pub async fn get_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<AccountRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        WHERE id = ?
        "#,
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    Ok(AccountRecord {
        id: row.get("id"),
        name: row.get("name"),
        account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
        base_currency: row.get("base_currency"),
        created_at: row.get("created_at"),
    })
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

struct AccountTotalSummary {
    status: AccountSummaryStatus,
    total_amount: Option<String>,
    total_currency: Option<String>,
}

async fn summarize_account(
    pool: &SqlitePool,
    account: &AccountRecord,
    balances: &[AccountBalanceRecord],
) -> Result<AccountTotalSummary, StorageError> {
    if balances.is_empty() {
        return Ok(AccountTotalSummary {
            status: AccountSummaryStatus::Ok,
            total_amount: Some("0.00000000".to_string()),
            total_currency: Some(account.base_currency.clone()),
        });
    }

    let mut total = Decimal::ZERO;

    for balance in balances {
        let amount = parse_stored_decimal(&balance.amount)?;

        if balance.currency == account.base_currency {
            total += amount;
            continue;
        }

        let Some(rate) =
            get_direct_fx_rate(pool, &balance.currency, &account.base_currency).await?
        else {
            return Ok(AccountTotalSummary {
                status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
                total_currency: None,
            });
        };

        total += amount * rate;
    }

    Ok(AccountTotalSummary {
        status: AccountSummaryStatus::Ok,
        total_amount: Some(crate::format_decimal_amount(total)),
        total_currency: Some(account.base_currency.clone()),
    })
}

async fn get_direct_fx_rate(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
) -> Result<Option<Decimal>, StorageError> {
    let rate = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(rate AS TEXT) AS rate
        FROM fx_rates
        WHERE from_currency = ? AND to_currency = ?
        "#,
    )
    .bind(from_currency)
    .bind(to_currency)
    .fetch_optional(pool)
    .await?;

    rate.map(|value| parse_stored_decimal(&value)).transpose()
}

fn parse_stored_decimal(value: &str) -> Result<Decimal, StorageError> {
    value
        .parse::<Decimal>()
        .map_err(|_| StorageError::Internal("stored decimal value is invalid"))
}

pub async fn delete_account_balance(
    pool: &SqlitePool,
    account_id: i64,
    currency: &str,
) -> Result<(), StorageError> {
    validate_allowed_currency(pool, currency).await?;

    let result = sqlx::query("DELETE FROM account_balances WHERE account_id = ? AND currency = ?")
        .bind(account_id)
        .bind(currency)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

pub async fn delete_account(pool: &SqlitePool, account_id: i64) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

fn validate_name(name: &str) -> Result<(), StorageError> {
    if name.trim().is_empty() {
        return Err(StorageError::Validation("name must not be empty"));
    }

    Ok(())
}

async fn validate_allowed_currency(pool: &SqlitePool, currency: &str) -> Result<(), StorageError> {
    let exists =
        sqlx::query_scalar::<_, i64>("SELECT EXISTS(SELECT 1 FROM currencies WHERE code = ?)")
            .bind(currency)
            .fetch_one(pool)
            .await?
            != 0;

    if !exists {
        return Err(StorageError::Validation(
            "currency must be one of: EUR, USD, GBP, CHF",
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

fn validate_positive_decimal(amount: &str) -> Result<(), StorageError> {
    if amount.starts_with('-')
        || amount == "0"
        || amount.starts_with("0.") && amount[2..].bytes().all(|byte| byte == b'0')
    {
        return Err(StorageError::Validation("rate must be greater than zero"));
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

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::{
        AccountBalanceRecord, AccountRecord, AccountSummaryRecord, AccountSummaryStatus,
        AccountType, CreateAccountInput, CurrencyRecord, FxRateRecord, StorageError,
        UpsertAccountBalanceInput, UpsertFxRateInput, UpsertOutcome, create_account,
        delete_account, delete_account_balance, get_account, list_account_balances,
        list_account_summaries, list_accounts, list_currencies, list_fx_rates,
        upsert_account_balance, upsert_fx_rate,
    };
    use crate::db::init_db;

    async fn test_pool() -> sqlx::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
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
    async fn lists_currencies_in_code_order() {
        let pool = test_pool().await;

        let currencies = list_currencies(&pool)
            .await
            .expect("currency list should succeed");

        assert_eq!(
            currencies,
            vec![
                CurrencyRecord {
                    code: "CHF".to_string(),
                },
                CurrencyRecord {
                    code: "EUR".to_string(),
                },
                CurrencyRecord {
                    code: "GBP".to_string(),
                },
                CurrencyRecord {
                    code: "USD".to_string(),
                },
            ]
        );
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
    async fn gets_single_account_by_id() {
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

        let account = get_account(&pool, account_id)
            .await
            .expect("single account fetch should succeed");

        assert_eq!(account.id, account_id);
        assert_eq!(account.name, "IBKR");
        assert_eq!(account.account_type, AccountType::Broker);
        assert_eq!(account.base_currency, "EUR");
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

        let first_outcome = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "10.00000000",
            },
        )
        .await
        .expect("first balance insert should succeed");
        assert_eq!(first_outcome, UpsertOutcome::Created);

        let second_outcome = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.00000000",
            },
        )
        .await
        .expect("upsert should update the existing balance");
        assert_eq!(second_outcome, UpsertOutcome::Updated);

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].amount, "12");
        assert_eq!(balances[0].updated_at.len(), 19);
    }

    #[tokio::test]
    async fn deletes_single_balance() {
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

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "EUR",
                amount: "12000.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        delete_account_balance(&pool, account_id, "EUR")
            .await
            .expect("balance delete should succeed");

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn deleting_missing_balance_returns_not_found() {
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

        let error = delete_account_balance(&pool, account_id, "USD")
            .await
            .expect_err("missing balance delete should fail");

        match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
    }

    #[tokio::test]
    async fn deletes_account_and_cascades_balances() {
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

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "EUR",
                amount: "12000.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        delete_account(&pool, account_id)
            .await
            .expect("account delete should succeed");

        let account_error = get_account(&pool, account_id)
            .await
            .expect_err("deleted account should not exist");
        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should still succeed");

        match account_error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn deleting_missing_account_returns_not_found() {
        let pool = test_pool().await;

        let error = delete_account(&pool, 999)
            .await
            .expect_err("missing account delete should fail");

        match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
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
            "currency must be one of: EUR, USD, GBP, CHF"
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
            "currency must be one of: EUR, USD, GBP, CHF"
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
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            StorageError::Database(error) => {
                assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
            }
            StorageError::Validation(_) | StorageError::Internal(_) => {
                panic!("expected database error")
            }
        }
    }

    #[tokio::test]
    async fn upserts_fx_rates() {
        let pool = test_pool().await;

        let outcome = upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: "USD",
                to_currency: "EUR",
                rate: "0.92000000",
            },
        )
        .await
        .expect("fx rate insert should succeed");

        assert_eq!(outcome, UpsertOutcome::Created);
        assert_eq!(
            list_fx_rates(&pool).await.expect("fx rates should list"),
            vec![FxRateRecord {
                from_currency: "USD".to_string(),
                to_currency: "EUR".to_string(),
                rate: "0.92".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn updates_existing_fx_rate() {
        let pool = test_pool().await;

        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: "USD",
                to_currency: "EUR",
                rate: "0.92000000",
            },
        )
        .await
        .expect("initial fx rate insert should succeed");

        let outcome = upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: "USD",
                to_currency: "EUR",
                rate: "0.91000000",
            },
        )
        .await
        .expect("fx rate update should succeed");

        assert_eq!(outcome, UpsertOutcome::Updated);
        assert_eq!(
            list_fx_rates(&pool).await.expect("fx rates should list"),
            vec![FxRateRecord {
                from_currency: "USD".to_string(),
                to_currency: "EUR".to_string(),
                rate: "0.91".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn rejects_non_positive_fx_rates() {
        let pool = test_pool().await;

        let error = upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: "USD",
                to_currency: "EUR",
                rate: "0.00000000",
            },
        )
        .await
        .expect_err("zero fx rate should fail");

        assert_eq!(error.to_string(), "rate must be greater than zero");
    }

    #[tokio::test]
    async fn lists_account_summaries_with_zero_total_for_empty_accounts() {
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

        assert_eq!(
            list_account_summaries(&pool)
                .await
                .expect("account summaries should succeed"),
            vec![AccountSummaryRecord {
                id: account_id,
                name: "IBKR".to_string(),
                account_type: AccountType::Broker,
                base_currency: "EUR".to_string(),
                summary_status: AccountSummaryStatus::Ok,
                total_amount: Some("0.00000000".to_string()),
                total_currency: Some("EUR".to_string()),
            }]
        );
    }

    #[tokio::test]
    async fn lists_account_summaries_with_single_base_currency_balance() {
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
                amount: "123.45000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
        assert_eq!(summaries[0].total_amount.as_deref(), Some("123.45000000"));
        assert_eq!(summaries[0].total_currency.as_deref(), Some("USD"));
    }

    #[tokio::test]
    async fn lists_account_summaries_with_direct_fx_conversion() {
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

        for (currency, amount) in [
            ("EUR", "10.00000000"),
            ("USD", "20.00000000"),
            ("GBP", "30.00000000"),
        ] {
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

        for (from_currency, rate) in [("USD", "0.50000000"), ("GBP", "1.20000000")] {
            upsert_fx_rate(
                &pool,
                UpsertFxRateInput {
                    from_currency,
                    to_currency: "EUR",
                    rate,
                },
            )
            .await
            .expect("fx rate insert should succeed");
        }

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
        assert_eq!(summaries[0].total_amount.as_deref(), Some("56.00000000"));
        assert_eq!(summaries[0].total_currency.as_deref(), Some("EUR"));
    }

    #[tokio::test]
    async fn marks_summary_unavailable_when_direct_fx_rate_is_missing() {
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

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "20.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(
            summaries[0],
            AccountSummaryRecord {
                id: account_id,
                name: "IBKR".to_string(),
                account_type: AccountType::Broker,
                base_currency: "EUR".to_string(),
                summary_status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
                total_currency: None,
            }
        );
    }

    #[tokio::test]
    async fn does_not_use_inverse_fx_rates() {
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

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "20.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: "EUR",
                to_currency: "USD",
                rate: "1.10000000",
            },
        )
        .await
        .expect("inverse fx rate insert should succeed");

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(
            summaries[0].summary_status,
            AccountSummaryStatus::ConversionUnavailable
        );
    }

    #[tokio::test]
    async fn does_not_use_multi_hop_fx_rates() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Swiss Cash",
                account_type: AccountType::Bank,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "CHF",
                amount: "20.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        for (from_currency, to_currency, rate) in
            [("CHF", "USD", "1.10000000"), ("USD", "EUR", "0.80000000")]
        {
            upsert_fx_rate(
                &pool,
                UpsertFxRateInput {
                    from_currency,
                    to_currency,
                    rate,
                },
            )
            .await
            .expect("multi-hop fx rate insert should succeed");
        }

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(
            summaries[0].summary_status,
            AccountSummaryStatus::ConversionUnavailable
        );
    }

    #[tokio::test]
    async fn rounds_after_summing_converted_balances() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Precise FX",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        for currency in ["USD", "GBP"] {
            upsert_account_balance(
                &pool,
                UpsertAccountBalanceInput {
                    account_id,
                    currency,
                    amount: "1.00000000",
                },
            )
            .await
            .expect("balance insert should succeed");
        }

        for (from_currency, rate) in [("USD", "0.33333333"), ("GBP", "0.33333333")] {
            upsert_fx_rate(
                &pool,
                UpsertFxRateInput {
                    from_currency,
                    to_currency: "EUR",
                    rate,
                },
            )
            .await
            .expect("fx rate insert should succeed");
        }

        let summaries = list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed");

        assert_eq!(summaries[0].total_amount.as_deref(), Some("0.66666666"));
    }
}
