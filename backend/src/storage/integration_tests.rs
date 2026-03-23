use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use super::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, CreateAccountInput, Currency, CurrencyRecord,
    FxRate, FxRateDetailRecord, FxRateRecord, FxRateSummaryItemRecord, FxRateSummaryRecord,
    StorageError, UpsertAccountBalanceInput, UpsertFxRateInput, UpsertOutcome, create_account,
    delete_account, delete_account_balance, get_account, get_latest_fx_rate, list_account_balances,
    list_account_summaries, list_accounts, list_currencies, list_fx_rate_summary, list_fx_rates,
    upsert_account_balance, upsert_fx_rate,
};
use crate::db::init_db;

fn amt(value: &str) -> Amount {
    Amount::try_from(value).expect("amount should parse")
}

fn fx_rate(value: &str) -> FxRate {
    FxRate::try_from(value).expect("rate should parse")
}

fn account_id(value: i64) -> AccountId {
    AccountId::try_from(value).expect("account id should parse")
}

fn account_name(value: &str) -> AccountName {
    AccountName::try_from(value).expect("account name should parse")
}

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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
                code: Currency::Chf,
            },
            CurrencyRecord {
                code: Currency::Eur,
            },
            CurrencyRecord {
                code: Currency::Gbp,
            },
            CurrencyRecord {
                code: Currency::Usd,
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
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("first account insert should succeed");

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("second account insert should succeed");

    let accounts = list_accounts(&pool)
        .await
        .expect("account list should succeed");

    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].name, account_name("Main Bank"));
    assert_eq!(accounts[0].account_type, AccountType::Bank);
    assert_eq!(accounts[1].name, account_name("IBKR"));
    assert_eq!(accounts[1].account_type, AccountType::Broker);
}

#[tokio::test]
async fn gets_single_account_by_id() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let account = get_account(&pool, account_id)
        .await
        .expect("single account fetch should succeed");

    assert_eq!(account.id, account_id);
    assert_eq!(account.name, account_name("IBKR"));
    assert_eq!(account.account_type, AccountType::Broker);
    assert_eq!(account.base_currency, Currency::Eur);
}

#[tokio::test]
async fn allows_multiple_currencies_per_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, value) in [("EUR", "12000.00000000"), ("USD", "3500.00000000")] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: Currency::try_from(currency).unwrap(),
                amount: amt(value),
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
                currency: Currency::Eur,
                amount: amt("12000"),
                updated_at: balances[0].updated_at.clone(),
            },
            AccountBalanceRecord {
                account_id,
                currency: Currency::Usd,
                amount: amt("3500"),
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
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let first_outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("10.00000000"),
        },
    )
    .await
    .expect("first balance insert should succeed");
    assert_eq!(first_outcome, UpsertOutcome::Created);

    let second_outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.00000000"),
        },
    )
    .await
    .expect("upsert should update the existing balance");
    assert_eq!(second_outcome, UpsertOutcome::Updated);

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].amount, amt("12"));
    assert_eq!(balances[0].updated_at.len(), 19);
}

#[tokio::test]
async fn deletes_single_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("12000.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    delete_account_balance(&pool, account_id, Currency::Eur)
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
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let error = delete_account_balance(&pool, account_id, Currency::Usd)
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("12000.00000000"),
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

    let error = delete_account(&pool, account_id(999))
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
            name: account_name("Joint Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Gbp,
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
            name: account_name("Joint Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Gbp,
            created_at: accounts[0].created_at.clone(),
        }]
    );
    assert_eq!(accounts[0].created_at.len(), 19);
}

#[tokio::test]
async fn rejects_invalid_account_type_input() {
    let error = AccountType::try_from("cash").expect_err("unsupported account type should fail");

    assert_eq!(
        error.to_string(),
        "account_type must be one of: bank, broker"
    );
}

#[tokio::test]
async fn accepts_typed_account_currency_input() {
    let pool = test_pool().await;

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("typed currency should succeed");
}

#[tokio::test]
async fn accepts_typed_balance_currency_input() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("10.00000000"),
        },
    )
    .await
    .expect("typed currency should succeed");

    assert_eq!(outcome, UpsertOutcome::Created);
}

#[test]
fn rejects_invalid_typed_amount_input() {
    let error = Amount::try_from("1.123456789").expect_err("invalid amount should fail");

    assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
}

#[tokio::test]
async fn rejects_balance_for_missing_account() {
    let pool = test_pool().await;

    let error = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id: account_id(999),
            currency: Currency::Usd,
            amount: amt("10.00000000"),
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
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92000000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    assert_eq!(outcome, UpsertOutcome::Created);
    assert_eq!(
        list_fx_rates(&pool).await.expect("fx rates should list"),
        vec![FxRateRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92"),
        }]
    );
}

#[tokio::test]
async fn gets_latest_fx_rate_with_timestamp() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92000000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    let rate = get_latest_fx_rate(&pool, Currency::Usd, Currency::Eur)
        .await
        .expect("fx lookup should succeed")
        .expect("fx rate should exist");

    assert_eq!(
        rate,
        FxRateDetailRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92"),
            updated_at: rate.updated_at.clone(),
        }
    );
    assert_eq!(rate.updated_at.len(), 19);
}

#[tokio::test]
async fn updates_existing_fx_rate() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92000000"),
        },
    )
    .await
    .expect("initial fx rate insert should succeed");

    let outcome = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.91000000"),
        },
    )
    .await
    .expect("fx rate update should succeed");

    assert_eq!(outcome, UpsertOutcome::Updated);
    assert_eq!(
        list_fx_rates(&pool).await.expect("fx rates should list"),
        vec![FxRateRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.91"),
        }]
    );
}

#[test]
fn rejects_non_positive_fx_rates() {
    let error = FxRate::try_from("0.00000000").expect_err("zero fx rate should fail");

    assert_eq!(error.to_string(), "rate must be greater than zero");
}

#[tokio::test]
async fn rejects_identity_fx_pairs() {
    let pool = test_pool().await;

    let error = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Eur,
            to_currency: Currency::Eur,
            rate: fx_rate("1.00000000"),
        },
    )
    .await
    .expect_err("identity fx pair should fail");

    assert_eq!(
        error.to_string(),
        "fx pair must contain two different currencies"
    );
}

#[tokio::test]
async fn rejects_non_eur_target_fx_pairs() {
    let pool = test_pool().await;

    let error = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Gbp,
            rate: fx_rate("0.78000000"),
        },
    )
    .await
    .expect_err("non-eur target pair should fail");

    assert_eq!(
        error.to_string(),
        "fx pair must convert a supported non-EUR currency into EUR"
    );
}

#[tokio::test]
async fn lists_fx_rate_summary_for_a_single_target_currency() {
    let pool = test_pool().await;

    for (from_currency, rate) in [
        (Currency::Usd, "0.92000000"),
        (Currency::Gbp, "1.17000000"),
        (Currency::Chf, "1.04000000"),
    ] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    for (from_currency, updated_at) in [
        ("USD", "2026-03-22 09:00:00"),
        ("GBP", "2026-03-22 10:00:00"),
        ("CHF", "2026-03-22 08:30:00"),
    ] {
        sqlx::query(
            "UPDATE fx_rates SET updated_at = ? WHERE from_currency = ? AND to_currency = 'EUR'",
        )
        .bind(updated_at)
        .bind(from_currency)
        .execute(&pool)
        .await
        .expect("timestamp update should succeed");
    }

    assert_eq!(
        list_fx_rate_summary(&pool, Currency::Eur)
            .await
            .expect("fx summary should succeed"),
        FxRateSummaryRecord {
            target_currency: Currency::Eur,
            rates: vec![
                FxRateSummaryItemRecord {
                    from_currency: Currency::Chf,
                    rate: fx_rate("1.04"),
                    updated_at: "2026-03-22 08:30:00".to_string(),
                },
                FxRateSummaryItemRecord {
                    from_currency: Currency::Gbp,
                    rate: fx_rate("1.17"),
                    updated_at: "2026-03-22 10:00:00".to_string(),
                },
                FxRateSummaryItemRecord {
                    from_currency: Currency::Usd,
                    rate: fx_rate("0.92"),
                    updated_at: "2026-03-22 09:00:00".to_string(),
                },
            ],
            last_updated: Some("2026-03-22 10:00:00".to_string()),
        }
    );
}

#[tokio::test]
async fn returns_empty_fx_rate_summary_when_target_has_no_rates() {
    let pool = test_pool().await;

    assert_eq!(
        list_fx_rate_summary(&pool, Currency::Eur)
            .await
            .expect("fx summary should succeed"),
        FxRateSummaryRecord {
            target_currency: Currency::Eur,
            rates: vec![],
            last_updated: None,
        }
    );
}

#[tokio::test]
async fn lists_account_summaries_with_zero_total_for_empty_accounts() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
            summary_status: AccountSummaryStatus::Ok,
            total_amount: Some(amt("0.00000000")),
            total_currency: Some(Currency::Eur),
        }]
    );
}

#[tokio::test]
async fn lists_account_summaries_with_single_base_currency_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("123.45000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].total_amount, Some(amt("123.45000000")));
    assert_eq!(summaries[0].total_currency, Some(Currency::Usd));
}

#[tokio::test]
async fn lists_account_summaries_with_direct_fx_conversion() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, value) in [
        (Currency::Eur, "10.00000000"),
        (Currency::Usd, "20.00000000"),
        (Currency::Gbp, "30.00000000"),
    ] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(value),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.50000000"), (Currency::Gbp, "1.20000000")] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].total_amount, Some(amt("56.00000000")));
    assert_eq!(summaries[0].total_currency, Some(Currency::Eur));
}

#[tokio::test]
async fn marks_summary_unavailable_when_direct_fx_rate_is_missing() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("20.00000000"),
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("20.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

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
            name: account_name("Swiss Cash"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Chf,
            amount: amt("20.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.80000000"),
        },
    )
    .await
    .expect("direct usd eur rate insert should succeed");

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
            name: account_name("Precise FX"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for currency in [Currency::Usd, Currency::Gbp] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt("1.00000000"),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.33333333"), (Currency::Gbp, "0.33333333")] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].total_amount, Some(amt("0.66666666")));
}

#[tokio::test]
async fn rejects_corrupt_account_rows_at_the_schema_level() {
    let pool = test_pool().await;

    let error = sqlx::query(
        "INSERT INTO accounts (name, account_type, base_currency, created_at) VALUES ('', 'bank', 'EUR', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("corrupt account row should be rejected");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn rejects_invalid_balance_amounts_at_the_schema_level() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let error = sqlx::query(
        "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, 'USD', '1.123456789', '2026-03-22 00:00:00')",
    )
    .bind(account_id.as_i64())
    .execute(&pool)
    .await
    .expect_err("invalid balance amount should be rejected");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn rejects_invalid_fx_rates_at_the_schema_level() {
    let pool = test_pool().await;

    let format_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '1.123456789', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("invalid fx rate format should be rejected");

    assert!(format_error.to_string().contains("CHECK constraint failed"));

    let zero_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '0.00000000', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("non-positive fx rate should be rejected");

    assert!(zero_error.to_string().contains("CHECK constraint failed"));
}
