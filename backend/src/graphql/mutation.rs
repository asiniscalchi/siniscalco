use std::collections::BTreeMap;

use async_graphql::{Context, ErrorExtensions, Name, Object, Value};
use sqlx::SqlitePool;
use tracing::warn;

use crate::{
    AccountId, AccountName, AccountType, Amount, AssetId, AssetName, AssetPriceRefreshConfig,
    AssetQuantity, AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice,
    CreateAccountInput, CreateAssetInput, CreateAssetTransactionInput, Currency, TradeDate,
    UpdateAccountInput, UpdateAssetInput, UpdateAssetTransactionInput, UpsertAccountBalanceInput,
    create_account, create_asset, create_asset_transaction, delete_account, delete_account_balance,
    delete_asset, delete_asset_transaction, get_account, get_account_value_summary, get_asset,
    list_account_balances, normalize_amount_output, refresh_single_asset_price,
    storage::StorageError, update_account, update_asset, update_asset_transaction,
    upsert_account_balance,
};

use super::{
    query::{storage_to_gql, to_account_detail, to_asset, to_transaction},
    types::*,
};

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_account(
        &self,
        ctx: &Context<'_>,
        name: String,
        account_type: String,
        base_currency: String,
    ) -> async_graphql::Result<AccountDetail> {
        let pool = ctx.data::<SqlitePool>()?;
        let name = AccountName::try_from(name.as_str()).map_err(storage_to_gql)?;
        let account_type = AccountType::try_from(account_type.as_str()).map_err(storage_to_gql)?;
        let base_currency = Currency::try_from(base_currency.as_str()).map_err(storage_to_gql)?;

        let account_id = create_account(
            pool,
            CreateAccountInput {
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
        name: String,
        account_type: String,
        base_currency: String,
    ) -> async_graphql::Result<AccountDetail> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(id).map_err(storage_to_gql)?;
        let name = AccountName::try_from(name.as_str()).map_err(storage_to_gql)?;
        let account_type = AccountType::try_from(account_type.as_str()).map_err(storage_to_gql)?;
        let base_currency = Currency::try_from(base_currency.as_str()).map_err(storage_to_gql)?;

        let account = update_account(
            pool,
            account_id,
            UpdateAccountInput {
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
        currency: String,
        amount: String,
    ) -> async_graphql::Result<Balance> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(account_id).map_err(storage_to_gql)?;
        let currency = Currency::try_from(currency.as_str()).map_err(storage_to_gql)?;
        let amount = Amount::try_from(amount.as_str()).map_err(storage_to_gql)?;

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
            UpsertAccountBalanceInput {
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
        symbol: String,
        name: String,
        asset_type: String,
        quote_symbol: Option<String>,
        isin: Option<String>,
    ) -> async_graphql::Result<Asset> {
        let pool = ctx.data::<SqlitePool>()?;
        let config = ctx.data::<AssetPriceRefreshConfig>()?;
        let input = validate_asset_input(symbol, name, asset_type, quote_symbol, isin)?;
        let asset_id = create_asset(pool, input)
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
        symbol: String,
        name: String,
        asset_type: String,
        quote_symbol: Option<String>,
        isin: Option<String>,
    ) -> async_graphql::Result<Asset> {
        let pool = ctx.data::<SqlitePool>()?;
        let config = ctx.data::<AssetPriceRefreshConfig>()?;
        let asset_id = AssetId::try_from(id).map_err(storage_to_gql)?;
        let input = validate_asset_input(symbol, name, asset_type, quote_symbol, isin)?;
        update_asset(
            pool,
            asset_id,
            UpdateAssetInput {
                symbol: input.symbol,
                name: input.name,
                asset_type: input.asset_type,
                quote_symbol: input.quote_symbol,
                isin: input.isin,
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
        account_id: i64,
        asset_id: i64,
        transaction_type: String,
        trade_date: String,
        quantity: String,
        unit_price: String,
        currency_code: String,
        notes: Option<String>,
    ) -> async_graphql::Result<Transaction> {
        let pool = ctx.data::<SqlitePool>()?;
        let input = parse_transaction_input(
            account_id,
            asset_id,
            transaction_type,
            trade_date,
            quantity,
            unit_price,
            currency_code,
            notes,
        )?;
        ensure_account_exists(pool, input.account_id).await?;
        ensure_asset_exists(pool, input.asset_id).await?;
        let tx = create_asset_transaction(pool, input)
            .await
            .map_err(storage_to_gql)?;
        Ok(to_transaction(tx))
    }

    async fn update_transaction(
        &self,
        ctx: &Context<'_>,
        id: i64,
        account_id: i64,
        asset_id: i64,
        transaction_type: String,
        trade_date: String,
        quantity: String,
        unit_price: String,
        currency_code: String,
        notes: Option<String>,
    ) -> async_graphql::Result<Transaction> {
        let pool = ctx.data::<SqlitePool>()?;
        let input = parse_transaction_input(
            account_id,
            asset_id,
            transaction_type,
            trade_date,
            quantity,
            unit_price,
            currency_code,
            notes,
        )?;
        ensure_account_exists(pool, input.account_id).await?;
        ensure_asset_exists(pool, input.asset_id).await?;
        let update_input = UpdateAssetTransactionInput {
            account_id: input.account_id,
            asset_id: input.asset_id,
            transaction_type: input.transaction_type,
            trade_date: input.trade_date,
            quantity: input.quantity,
            unit_price: input.unit_price,
            currency_code: input.currency_code,
            notes: input.notes,
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

fn validate_asset_input(
    symbol: String,
    name: String,
    asset_type: String,
    quote_symbol: Option<String>,
    isin: Option<String>,
) -> async_graphql::Result<CreateAssetInput> {
    let mut field_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        field_errors.insert("symbol".to_string(), vec!["Symbol is required".to_string()]);
    }

    let name = name.trim().to_string();
    if name.is_empty() {
        field_errors.insert("name".to_string(), vec!["Name is required".to_string()]);
    }

    let asset_type_str = asset_type.trim().to_string();
    if asset_type_str.is_empty() {
        field_errors.insert(
            "assetType".to_string(),
            vec!["Asset type is required".to_string()],
        );
    }

    let quote_symbol = quote_symbol.and_then(|s| {
        let t = s.trim().to_uppercase();
        (!t.is_empty()).then_some(t)
    });
    let isin = isin.and_then(|s| {
        let t = s.trim().to_string();
        (!t.is_empty()).then_some(t)
    });

    let parsed_asset_type =
        match AssetType::try_from(asset_type_str.as_str()) {
            Ok(t) => Some(t),
            Err(_) if !asset_type_str.is_empty() => {
                field_errors.insert(
                "assetType".to_string(),
                vec!["Asset type must be one of: STOCK, ETF, BOND, CRYPTO, CASH_EQUIVALENT, OTHER"
                    .to_string()],
            );
                None
            }
            Err(_) => None,
        };

    if !field_errors.is_empty() {
        let val = field_errors_to_value(field_errors);
        return Err(async_graphql::Error::new("Asset validation failed")
            .extend_with(|_, e| e.set("field_errors", val)));
    }

    Ok(CreateAssetInput {
        symbol: AssetSymbol::try_from(symbol.as_str()).map_err(storage_to_gql)?,
        name: AssetName::try_from(name.as_str()).map_err(storage_to_gql)?,
        asset_type: parsed_asset_type.expect("validated"),
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
    account_id: i64,
    asset_id: i64,
    transaction_type: String,
    trade_date: String,
    quantity: String,
    unit_price: String,
    currency_code: String,
    notes: Option<String>,
) -> async_graphql::Result<CreateAssetTransactionInput> {
    let account_id = AccountId::try_from(account_id).map_err(storage_to_gql)?;
    let asset_id = AssetId::try_from(asset_id).map_err(storage_to_gql)?;
    let transaction_type =
        AssetTransactionType::try_from(transaction_type.as_str()).map_err(storage_to_gql)?;
    let trade_date = TradeDate::try_from(trade_date.as_str()).map_err(storage_to_gql)?;
    let quantity = AssetQuantity::try_from(quantity.as_str()).map_err(storage_to_gql)?;
    let unit_price = AssetUnitPrice::try_from(unit_price.as_str()).map_err(storage_to_gql)?;
    let currency_code = Currency::try_from(currency_code.as_str()).map_err(storage_to_gql)?;

    Ok(CreateAssetTransactionInput {
        account_id,
        asset_id,
        transaction_type,
        trade_date,
        quantity,
        unit_price,
        currency_code,
        notes,
    })
}

async fn ensure_account_exists(
    pool: &SqlitePool,
    account_id: AccountId,
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

async fn ensure_asset_exists(pool: &SqlitePool, asset_id: AssetId) -> async_graphql::Result<()> {
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
    asset_id: AssetId,
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
