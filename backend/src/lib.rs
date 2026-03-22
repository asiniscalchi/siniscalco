use sqlx::{Executor, SqlitePool};

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('bank', 'broker')),
    base_currency TEXT NOT NULL CHECK (base_currency GLOB '[A-Z][A-Z][A-Z]'),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS account_balances (
    account_id INTEGER NOT NULL,
    currency TEXT NOT NULL CHECK (currency GLOB '[A-Z][A-Z][A-Z]'),
    amount DECIMAL(20,8) NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (account_id, currency),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
"#;

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    pool.execute("PRAGMA foreign_keys = ON;").await?;
    pool.execute(SCHEMA_SQL).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::init_db;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite pool should connect");

        init_db(&pool).await.expect("schema should initialize");
        pool
    }

    #[tokio::test]
    async fn creates_account_without_balance() {
        let pool = test_pool().await;

        sqlx::query("INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)")
            .bind("IBKR")
            .bind("broker")
            .bind("EUR")
            .execute(&pool)
            .await
            .expect("account insert should succeed");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .expect("account count query should succeed");

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn allows_multiple_currencies_per_account() {
        let pool = test_pool().await;

        let account_id = sqlx::query(
            "INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)",
        )
        .bind("IBKR")
        .bind("broker")
        .bind("EUR")
        .execute(&pool)
        .await
        .expect("account insert should succeed")
        .last_insert_rowid();

        for (currency, amount) in [("EUR", "12000.00000000"), ("USD", "3500.00000000")] {
            sqlx::query(
                "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, ?, ?, ?)",
            )
            .bind(account_id)
            .bind(currency)
            .bind(amount)
            .bind("2026-03-22 00:00:00")
            .execute(&pool)
            .await
            .expect("balance insert should succeed");
        }

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM account_balances WHERE account_id = ?")
                .bind(account_id)
                .fetch_one(&pool)
                .await
                .expect("balance count query should succeed");

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn rejects_duplicate_balance_currency_per_account() {
        let pool = test_pool().await;

        let account_id = sqlx::query(
            "INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)",
        )
        .bind("Main Bank")
        .bind("bank")
        .bind("USD")
        .execute(&pool)
        .await
        .expect("account insert should succeed")
        .last_insert_rowid();

        sqlx::query(
            "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind("USD")
        .bind("10.00000000")
        .bind("2026-03-22 00:00:00")
        .execute(&pool)
        .await
        .expect("first balance insert should succeed");

        let error = sqlx::query(
            "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind("USD")
        .bind("12.00000000")
        .bind("2026-03-22 00:01:00")
        .execute(&pool)
        .await
        .expect_err("duplicate balance insert should fail");

        assert!(error.to_string().contains("UNIQUE"));
    }

    #[tokio::test]
    async fn rejects_invalid_account_type() {
        let pool = test_pool().await;

        let error = sqlx::query(
            "INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)",
        )
        .bind("Cash Jar")
        .bind("cash")
        .bind("EUR")
        .execute(&pool)
        .await
        .expect_err("invalid account type should fail");

        assert!(error.to_string().contains("CHECK constraint failed"));
    }

    #[tokio::test]
    async fn rejects_invalid_currency_format() {
        let pool = test_pool().await;

        let error = sqlx::query(
            "INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)",
        )
        .bind("Main Bank")
        .bind("bank")
        .bind("usd")
        .execute(&pool)
        .await
        .expect_err("lowercase currency should fail");

        assert!(error.to_string().contains("CHECK constraint failed"));
    }

    #[tokio::test]
    async fn rejects_balance_for_missing_account() {
        let pool = test_pool().await;

        let error = sqlx::query(
            "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(999_i64)
        .bind("USD")
        .bind("10.00000000")
        .bind("2026-03-22 00:00:00")
        .execute(&pool)
        .await
        .expect_err("missing parent account should fail");

        assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
    }
}
