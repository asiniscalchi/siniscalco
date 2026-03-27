use std::collections::BTreeMap;

use async_graphql::{Context, ErrorExtensions, Name, Object, Value};
use sqlx::SqlitePool;
use tracing::warn;

use crate::{
    AccountId, AccountName, Amount, AssetId, AssetName, AssetPriceRefreshConfig, AssetQuantity,
    AssetSymbol, AssetUnitPrice, Currency, TradeDate, delete_account, delete_account_balance,
    delete_asset, delete_asset_transaction, get_account, get_account_value_summary, get_asset,
    list_account_balances, normalize_amount_output, refresh_single_asset_price,
    storage::StorageError, update_account, update_asset, update_asset_transaction,
    upsert_account_balance,
};

use super::{
    query::{storage_to_gql, to_account_detail, to_asset, to_transaction},
    types::{
        AccountDetail, Asset, Balance, CreateAccountInput, CreateAssetInput,
        CreateTransactionInput, UpdateAccountInput, UpdateAssetInput, UpdateTransactionInput,
        UpsertBalanceInput,
    },
};

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_account(
        &self,
        ctx: &Context<'_>,
        input: CreateAccountInput,
    ) -> async_graphql::Result<AccountDetail> {
        let pool = ctx.data::<SqlitePool>()?;
        let name = AccountName::try_from(input.name.as_str()).map_err(storage_to_gql)?;
        let account_type: crate::AccountType = input.account_type.into();
        let base_currency =
            Currency::try_from(input.base_currency.as_str()).map_err(storage_to_gql)?;

        let account_id = crate::create_account(
            pool,
            crate::CreateAccountInput {
                name,
                account_type,
                base_currency,
            },
        )
        .await
        .map_err(storage_to_gql)?;
        let account = get_account(pool, account_id)
            .await
            .map_err(storage_to_gql)?;
        let balances = list_account_balances(pool, account_id)
            .await
            .map_err(storage_to_gql)?;
        let value_summary = get_account_value_summary(pool, &account)
            .await
            .map_err(storage_to_gql)?;

        Ok(to_account_detail(account, balances, value_summary))
    }

    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: UpdateAccountInput,
    ) -> async_graphql::Result<AccountDetail> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(id).map_err(storage_to_gql)?;
        let name = AccountName::try_from(input.name.as_str()).map_err(storage_to_gql)?;
        let account_type: crate::AccountType = input.account_type.into();
        let base_currency =
            Currency::try_from(input.base_currency.as_str()).map_err(storage_to_gql)?;

        let account = update_account(
            pool,
            account_id,
            crate::UpdateAccountInput {
                name,
                account_type,
                base_currency,
            },
        )
        .await
        .map_err(|err| match err {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                async_graphql::Error::new("Account not found")
            }
            other => storage_to_gql(other),
        })?;
        let balances = list_account_balances(pool, account_id)
            .await
            .map_err(storage_to_gql)?;
        let value_summary = get_account_value_summary(pool, &account)
            .await
            .map_err(storage_to_gql)?;

        Ok(to_account_detail(account, balances, value_summary))
    }

    async fn delete_account(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(id).map_err(storage_to_gql)?;
        delete_account(pool, account_id)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Account not found")
                }
                other => storage_to_gql(other),
            })?;
        Ok(true)
    }

    async fn upsert_balance(
        &self,
        ctx: &Context<'_>,
        account_id: i64,
        input: UpsertBalanceInput,
    ) -> async_graphql::Result<Balance> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(account_id).map_err(storage_to_gql)?;
        let currency = Currency::try_from(input.currency.as_str()).map_err(storage_to_gql)?;
        let amount = Amount::try_from(input.amount.as_str()).map_err(storage_to_gql)?;

        get_account(pool, account_id)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Account not found")
                }
                other => storage_to_gql(other),
            })?;

        upsert_account_balance(
            pool,
            crate::UpsertAccountBalanceInput {
                account_id,
                currency,
                amount,
            },
        )
        .await
        .map_err(storage_to_gql)?;

        let balance = list_account_balances(pool, account_id)
            .await
            .map_err(storage_to_gql)?
            .into_iter()
            .find(|b| b.currency == currency)
            .ok_or_else(|| async_graphql::Error::new("Internal server error"))?;

        Ok(Balance {
            currency: balance.currency.as_str().to_string(),
            amount: normalize_amount_output(&balance.amount.to_string()),
            updated_at: balance.updated_at,
        })
    }

    async fn delete_balance(
        &self,
        ctx: &Context<'_>,
        account_id: i64,
        currency: String,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(account_id).map_err(storage_to_gql)?;
        let currency = Currency::try_from(currency.as_str()).map_err(storage_to_gql)?;
        delete_account_balance(pool, account_id, currency)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Balance not found")
                }
                other => storage_to_gql(other),
            })?;
        Ok(true)
    }

    async fn create_asset(
        &self,
        ctx: &Context<'_>,
        input: CreateAssetInput,
    ) -> async_graphql::Result<Asset> {
        let pool = ctx.data::<SqlitePool>()?;
        let config = ctx.data::<AssetPriceRefreshConfig>()?;
        let storage_input = validate_asset_input(input)?;
        let asset_id = crate::create_asset(pool, storage_input)
            .await
            .map_err(asset_storage_error)?;
        refresh_asset_price(pool, config, asset_id).await;
        let asset = get_asset(pool, asset_id).await.map_err(storage_to_gql)?;
        Ok(to_asset(asset))
    }

    async fn update_asset(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: UpdateAssetInput,
    ) -> async_graphql::Result<Asset> {
        let pool = ctx.data::<SqlitePool>()?;
        let config = ctx.data::<AssetPriceRefreshConfig>()?;
        let asset_id = AssetId::try_from(id).map_err(storage_to_gql)?;
        let storage_input = validate_asset_input(CreateAssetInput {
            symbol: input.symbol,
            name: input.name,
            asset_type: input.asset_type,
            quote_symbol: input.quote_symbol,
            isin: input.isin,
        })?;
        update_asset(
            pool,
            asset_id,
            crate::UpdateAssetInput {
                symbol: storage_input.symbol,
                name: storage_input.name,
                asset_type: storage_input.asset_type,
                quote_symbol: storage_input.quote_symbol,
                isin: storage_input.isin,
            },
        )
        .await
        .map_err(|err| match err {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                async_graphql::Error::new("Asset not found")
            }
            other => asset_storage_error(other),
        })?;
        refresh_asset_price(pool, config, asset_id).await;
        let asset = get_asset(pool, asset_id).await.map_err(storage_to_gql)?;
        Ok(to_asset(asset))
    }

    async fn delete_asset(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let asset_id = AssetId::try_from(id).map_err(storage_to_gql)?;
        delete_asset(pool, asset_id)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Asset not found")
                }
                StorageError::Database(sqlx::Error::Database(db_err))
                    if db_err.message().contains("FOREIGN KEY constraint failed") =>
                {
                    async_graphql::Error::new("Asset has transactions and cannot be deleted")
                }
                other => storage_to_gql(other),
            })?;
        Ok(true)
    }

    async fn create_transaction(
        &self,
        ctx: &Context<'_>,
        input: CreateTransactionInput,
    ) -> async_graphql::Result<super::types::Transaction> {
        let pool = ctx.data::<SqlitePool>()?;
        let storage_input = parse_transaction_input(input)?;
        ensure_account_exists(pool, storage_input.account_id).await?;
        ensure_asset_exists(pool, storage_input.asset_id).await?;
        let tx = crate::create_asset_transaction(pool, storage_input)
            .await
            .map_err(storage_to_gql)?;
        Ok(to_transaction(tx))
    }

    async fn update_transaction(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: UpdateTransactionInput,
    ) -> async_graphql::Result<super::types::Transaction> {
        let pool = ctx.data::<SqlitePool>()?;
        let storage_input = parse_transaction_input(CreateTransactionInput {
            account_id: input.account_id,
            asset_id: input.asset_id,
            transaction_type: input.transaction_type,
            trade_date: input.trade_date,
            quantity: input.quantity,
            unit_price: input.unit_price,
            currency_code: input.currency_code,
            notes: input.notes,
        })?;
        ensure_account_exists(pool, storage_input.account_id).await?;
        ensure_asset_exists(pool, storage_input.asset_id).await?;
        let update_input = crate::UpdateAssetTransactionInput {
            account_id: storage_input.account_id,
            asset_id: storage_input.asset_id,
            transaction_type: storage_input.transaction_type,
            trade_date: storage_input.trade_date,
            quantity: storage_input.quantity,
            unit_price: storage_input.unit_price,
            currency_code: storage_input.currency_code,
            notes: storage_input.notes,
        };
        let tx = update_asset_transaction(pool, id, update_input)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Transaction not found")
                }
                other => storage_to_gql(other),
            })?;
        Ok(to_transaction(tx))
    }

    async fn delete_transaction(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        delete_asset_transaction(pool, id)
            .await
            .map_err(|err| match err {
                StorageError::Database(sqlx::Error::RowNotFound) => {
                    async_graphql::Error::new("Transaction not found")
                }
                other => storage_to_gql(other),
            })?;
        Ok(true)
    }
}

fn validate_asset_input(input: CreateAssetInput) -> async_graphql::Result<crate::CreateAssetInput> {
    let mut field_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let symbol = input.symbol.trim().to_uppercase();
    if symbol.is_empty() {
        field_errors.insert("symbol".to_string(), vec!["Symbol is required".to_string()]);
    }

    let name = input.name.trim().to_string();
    if name.is_empty() {
        field_errors.insert("name".to_string(), vec!["Name is required".to_string()]);
    }

    let quote_symbol = input.quote_symbol.and_then(|s| {
        let t = s.trim().to_uppercase();
        (!t.is_empty()).then_some(t)
    });
    let isin = input.isin.and_then(|s| {
        let t = s.trim().to_string();
        (!t.is_empty()).then_some(t)
    });

    if !field_errors.is_empty() {
        let val = field_errors_to_value(field_errors);
        return Err(async_graphql::Error::new("Asset validation failed")
            .extend_with(|_, e| e.set("field_errors", val)));
    }

    Ok(crate::CreateAssetInput {
        symbol: AssetSymbol::try_from(symbol.as_str()).map_err(storage_to_gql)?,
        name: AssetName::try_from(name.as_str()).map_err(storage_to_gql)?,
        asset_type: input.asset_type.into(),
        quote_symbol,
        isin,
    })
}

fn asset_storage_error(err: StorageError) -> async_graphql::Error {
    match err {
        StorageError::Database(sqlx::Error::Database(db_err)) => {
            let msg = db_err.message();
            if msg.contains("UNIQUE constraint failed: assets.symbol") {
                let val = single_field_error("symbol", "Symbol must be unique");
                return async_graphql::Error::new("Asset validation failed")
                    .extend_with(|_, e| e.set("field_errors", val));
            }
            if msg.contains("UNIQUE constraint failed: assets.isin") {
                let val = single_field_error("isin", "ISIN must be unique");
                return async_graphql::Error::new("Asset validation failed")
                    .extend_with(|_, e| e.set("field_errors", val));
            }
            if msg.contains("UNIQUE constraint failed: assets.quote_symbol") {
                let val = single_field_error("quote_symbol", "Quote symbol must be unique");
                return async_graphql::Error::new("Asset validation failed")
                    .extend_with(|_, e| e.set("field_errors", val));
            }
            async_graphql::Error::new("Internal server error")
        }
        other => storage_to_gql(other),
    }
}

fn field_errors_to_value(field_errors: BTreeMap<String, Vec<String>>) -> Value {
    Value::Object(
        field_errors
            .into_iter()
            .map(|(k, v)| {
                let list = Value::List(v.into_iter().map(Value::String).collect());
                (Name::new(k), list)
            })
            .collect(),
    )
}

fn single_field_error(field: &str, message: &str) -> Value {
    Value::Object(
        [(
            Name::new(field),
            Value::List(vec![Value::String(message.to_string())]),
        )]
        .into_iter()
        .collect(),
    )
}

fn parse_transaction_input(
    input: CreateTransactionInput,
) -> async_graphql::Result<crate::CreateAssetTransactionInput> {
    let account_id = AccountId::try_from(input.account_id).map_err(storage_to_gql)?;
    let asset_id = AssetId::try_from(input.asset_id).map_err(storage_to_gql)?;
    let trade_date = TradeDate::try_from(input.trade_date.as_str()).map_err(storage_to_gql)?;
    let quantity = AssetQuantity::try_from(input.quantity.as_str()).map_err(storage_to_gql)?;
    let unit_price = AssetUnitPrice::try_from(input.unit_price.as_str()).map_err(storage_to_gql)?;
    let currency_code = Currency::try_from(input.currency_code.as_str()).map_err(storage_to_gql)?;

    Ok(crate::CreateAssetTransactionInput {
        account_id,
        asset_id,
        transaction_type: input.transaction_type.into(),
        trade_date,
        quantity,
        unit_price,
        currency_code,
        notes: input.notes,
    })
}

async fn ensure_account_exists(
    pool: &SqlitePool,
    account_id: crate::AccountId,
) -> async_graphql::Result<()> {
    get_account(pool, account_id)
        .await
        .map(|_| ())
        .map_err(|err| match err {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                async_graphql::Error::new("Account not found")
            }
            other => storage_to_gql(other),
        })
}

async fn ensure_asset_exists(
    pool: &SqlitePool,
    asset_id: crate::AssetId,
) -> async_graphql::Result<()> {
    get_asset(pool, asset_id)
        .await
        .map(|_| ())
        .map_err(|err| match err {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                async_graphql::Error::new("Asset not found")
            }
            other => storage_to_gql(other),
        })
}

async fn refresh_asset_price(
    pool: &SqlitePool,
    config: &AssetPriceRefreshConfig,
    asset_id: crate::AssetId,
) {
    let client = reqwest::Client::new();
    if let Err(error) = refresh_single_asset_price(pool, &client, config, asset_id).await {
        warn!(
            asset_id = asset_id.as_i64(),
            error = %error,
            "failed to refresh immediate asset price"
        );
    }
}
