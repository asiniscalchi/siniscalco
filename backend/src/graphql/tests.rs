use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tower::ServiceExt;

use super::{AppState, build_router, build_router_with_state};
use crate::{
    AccountName, AccountType, Amount, AssetName, AssetPriceRefreshConfig, AssetQuantity,
    AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice, CreateAccountInput,
    CreateAssetInput, CreateAssetTransactionInput, Currency, FxRate, FxRefreshAvailability,
    FxRefreshStatus, TradeDate, UpsertAccountBalanceInput, UpsertAssetPriceInput,
    UpsertFxRateInput, assistant::new_assistant_chat_semaphore,
    assistant::new_shared_assistant_model_registry, assistant::refresh_assistant_model_registry,
    create_account, create_asset, create_asset_transaction, init_db, upsert_account_balance,
    upsert_asset_price, upsert_fx_rate,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn amt(value: &str) -> Amount {
    Amount::try_from(value).expect("amount should parse")
}

fn fx_rate(value: &str) -> FxRate {
    FxRate::try_from(value).expect("rate should parse")
}

fn account_name(value: &str) -> AccountName {
    AccountName::try_from(value).expect("account name should parse")
}

fn asset_symbol(value: &str) -> AssetSymbol {
    AssetSymbol::try_from(value).expect("asset symbol should parse")
}

fn asset_name(value: &str) -> AssetName {
    AssetName::try_from(value).expect("asset name should parse")
}

fn trade_date(value: &str) -> TradeDate {
    TradeDate::try_from(value).expect("trade date should parse")
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

fn no_price_config() -> AssetPriceRefreshConfig {
    AssetPriceRefreshConfig {
        refresh_interval: std::time::Duration::from_secs(60),
        coingecko_base_url: "http://127.0.0.1:1".to_string(),
        coincap_base_url: "http://127.0.0.1:1".to_string(),
        coincap_api_key: None,
        openfigi_base_url: "http://127.0.0.1:1".to_string(),
        openfigi_api_key: None,
        twelve_data_base_url: "http://127.0.0.1:1".to_string(),
        twelve_data_api_key: None,
        finnhub_base_url: "http://127.0.0.1:1".to_string(),
        finnhub_api_key: None,
        alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
        alpha_vantage_api_key: None,
        polygon_base_url: "http://127.0.0.1:1".to_string(),
        polygon_api_key: None,
        fmp_base_url: "http://127.0.0.1:1".to_string(),
        fmp_api_key: None,
        eodhd_base_url: "http://127.0.0.1:1".to_string(),
        eodhd_api_key: None,
        tiingo_base_url: "http://127.0.0.1:1".to_string(),
        tiingo_api_key: None,
        marketstack_base_url: "http://127.0.0.1:1".to_string(),
        marketstack_api_key: None,
    }
}

fn build_app_with_fx_status(
    pool: sqlx::SqlitePool,
    availability: FxRefreshAvailability,
    last_error: Option<&str>,
) -> Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus {
            availability,
            last_error: last_error.map(str::to_string),
        })),
        asset_price_refresh_config: no_price_config(),
        http_client: reqwest::Client::new(),
        openai_api_key: None,
        assistant_models: new_shared_assistant_model_registry(None),
        assistant_chat_semaphore: new_assistant_chat_semaphore(),
        openai_chat_url: crate::assistant::openai_chat_url().to_string(),
        openai_models_url: crate::assistant::openai_models_url().to_string(),
    })
}

fn build_app_with_price_config(pool: sqlx::SqlitePool, config: AssetPriceRefreshConfig) -> Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus::available())),
        asset_price_refresh_config: config,
        http_client: reqwest::Client::new(),
        openai_api_key: None,
        assistant_models: new_shared_assistant_model_registry(None),
        assistant_chat_semaphore: new_assistant_chat_semaphore(),
        openai_chat_url: crate::assistant::openai_chat_url().to_string(),
        openai_models_url: crate::assistant::openai_models_url().to_string(),
    })
}

fn build_app_with_openai(
    pool: sqlx::SqlitePool,
    api_key: Option<&str>,
    openai_chat_url: String,
    openai_models_url: String,
) -> Router {
    build_app_with_openai_registry(
        pool,
        api_key,
        openai_chat_url,
        openai_models_url,
        new_shared_assistant_model_registry(api_key),
    )
}

fn build_app_with_openai_registry(
    pool: sqlx::SqlitePool,
    api_key: Option<&str>,
    openai_chat_url: String,
    openai_models_url: String,
    assistant_models: crate::assistant::SharedAssistantModelRegistry,
) -> Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus::available())),
        asset_price_refresh_config: no_price_config(),
        http_client: reqwest::Client::new(),
        openai_api_key: api_key.map(str::to_string),
        assistant_models,
        assistant_chat_semaphore: new_assistant_chat_semaphore(),
        openai_chat_url,
        openai_models_url,
    })
}

async fn gql(app: Router, query: &str) -> Value {
    let body = json!({ "query": query });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post_json(app: Router, path: &str, body: Value) -> (StatusCode, Value) {
    send_json(app, "POST", path, body).await
}

async fn put_json(app: Router, path: &str, body: Value) -> (StatusCode, Value) {
    send_json(app, "PUT", path, body).await
}

async fn send_json(app: Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();

    (status, json)
}

async fn get_json(app: Router, path: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();

    (status, json)
}

async fn start_test_quote_server(payload: Value) -> String {
    let app = Router::new().route(
        "/quote",
        get(move || {
            let payload = payload.clone();
            async move { Json(payload) }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should expose addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });
    format!("http://{address}")
}

async fn start_test_openai_error_server(status: StatusCode, payload: Value) -> String {
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let payload = payload.clone();
            async move { (status, Json(payload)) }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should expose addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });
    format!("http://{address}/v1/chat/completions")
}

async fn start_test_openai_models_server(payload: Value) -> String {
    let app = Router::new().route(
        "/v1/models",
        get(move || {
            let payload = payload.clone();
            async move { Json(payload) }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should expose addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });
    format!("http://{address}/v1/models")
}

async fn start_test_openai_tool_server(recorded_requests: Arc<Mutex<Vec<Value>>>) -> String {
    let request_count = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |Json(body): Json<Value>| {
            let recorded_requests = Arc::clone(&recorded_requests);
            let request_count = Arc::clone(&request_count);

            async move {
                recorded_requests.lock().await.push(body);

                let response = match request_count.fetch_add(1, Ordering::SeqCst) {
                    0 => json!({
                        "choices": [
                            {
                                "finish_reason": "tool_calls",
                                "message": {
                                    "role": "assistant",
                                    "content": null,
                                    "tool_calls": [
                                        {
                                            "id": "call_1",
                                            "type": "function",
                                            "function": {
                                                "name": "list_accounts",
                                                "arguments": "{}"
                                            }
                                        }
                                    ]
                                }
                            }
                        ]
                    }),
                    _ => json!({
                        "choices": [
                            {
                                "finish_reason": "stop",
                                "message": {
                                    "role": "assistant",
                                    "content": [
                                        { "type": "text", "text": "You have 1 account." }
                                    ]
                                }
                            }
                        ]
                    }),
                };

                Json(response)
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should expose addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });
    format!("http://{address}/v1/chat/completions")
}

// ── health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn serves_health_route() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn assistant_chat_returns_db_backed_portfolio_summary() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Broker"),
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
            amount: amt("125.50"),
        },
    )
    .await
    .expect("balance upsert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({
            "messages": [
                { "role": "user", "content": "What does my portfolio look like?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let message = json["message"]
        .as_str()
        .expect("assistant response should include a message");
    assert!(message.contains("125.5 EUR"));
    assert!(message.contains("1 account"));
}

#[tokio::test]
async fn assistant_chat_handles_empty_prompt_with_backend_status_summary() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({
            "messages": []
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let message = json["message"]
        .as_str()
        .expect("assistant response should include a message");
    assert!(message.contains("backend assistant is connected"));
    assert!(message.contains("0 account"));
}

#[tokio::test]
async fn assistant_models_returns_mock_backend_when_openai_is_disabled() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let (status, json) = get_json(app, "/assistant/models").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["models"], json!(["mock-backend"]));
    assert_eq!(json["selected_model"], "mock-backend");
    assert_eq!(json["openai_enabled"], false);
}

#[tokio::test]
async fn assistant_models_exposes_refreshed_openai_model_list() {
    let pool = test_pool().await;
    let models_url = start_test_openai_models_server(json!({
        "data": [
            { "id": "another-model" },
            { "id": "gpt-4.1-mini" },
            { "id": "gpt-4o-mini" },
            { "id": "gpt-4.1" },
            { "id": "not-allowed-model" }
        ]
    }))
    .await;
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"));
    refresh_assistant_model_registry(
        &assistant_models,
        &reqwest::Client::new(),
        Some("test-key"),
        &models_url,
    )
    .await
    .expect("assistant models should refresh");

    let app = build_app_with_openai_registry(
        pool,
        Some("test-key"),
        crate::assistant::openai_chat_url().to_string(),
        models_url,
        assistant_models,
    );
    let (status, json) = get_json(app, "/assistant/models").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json["models"],
        json!([
            "another-model",
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4o-mini",
            "not-allowed-model"
        ])
    );
    assert_eq!(json["selected_model"], "gpt-4o-mini");
    assert_eq!(json["openai_enabled"], true);
    assert!(json["last_refreshed_at"].is_string());
}

#[tokio::test]
async fn assistant_models_selection_updates_in_memory_model_used_by_chat() {
    let pool = test_pool().await;
    let recorded_requests = Arc::new(Mutex::new(Vec::new()));
    let openai_chat_url = start_test_openai_tool_server(Arc::clone(&recorded_requests)).await;
    let models_url = start_test_openai_models_server(json!({
        "data": [
            { "id": "gpt-4.1-mini" },
            { "id": "gpt-4o-mini" }
        ]
    }))
    .await;
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"));
    refresh_assistant_model_registry(
        &assistant_models,
        &reqwest::Client::new(),
        Some("test-key"),
        &models_url,
    )
    .await
    .expect("assistant models should refresh");

    let app = build_app_with_openai_registry(
        pool,
        Some("test-key"),
        openai_chat_url,
        models_url,
        assistant_models,
    );
    let (status, json) = put_json(
        app.clone(),
        "/assistant/models/selected",
        json!({ "model": "gpt-4.1-mini" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["selected_model"], "gpt-4.1-mini");

    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({
            "messages": [
                { "role": "user", "content": "How many accounts do I have?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["model"], "gpt-4.1-mini");

    let recorded_requests = recorded_requests.lock().await;
    assert_eq!(recorded_requests[0]["model"], "gpt-4.1-mini");
}

#[tokio::test]
async fn assistant_chat_surfaces_openai_failures_as_bad_gateway() {
    let pool = test_pool().await;
    let openai_chat_url = start_test_openai_error_server(
        StatusCode::UNAUTHORIZED,
        json!({
            "error": {
                "message": "Incorrect API key provided"
            }
        }),
    )
    .await;
    let app = build_app_with_openai(
        pool,
        Some("test-key"),
        openai_chat_url,
        crate::assistant::openai_models_url().to_string(),
    );
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({
            "messages": [
                { "role": "user", "content": "What does my portfolio look like?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let error = json["error"]
        .as_str()
        .expect("assistant error response should include a message");
    assert!(error.contains("OpenAI 401 Unauthorized"));
    assert!(error.contains("Incorrect API key provided"));
}

#[tokio::test]
async fn assistant_chat_completes_tool_call_round_trip_against_openai() {
    let pool = test_pool().await;

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let recorded_requests = Arc::new(Mutex::new(Vec::new()));
    let openai_chat_url = start_test_openai_tool_server(Arc::clone(&recorded_requests)).await;
    let app = build_app_with_openai(
        pool,
        Some("test-key"),
        openai_chat_url,
        crate::assistant::openai_models_url().to_string(),
    );
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({
            "messages": [
                { "role": "user", "content": "How many accounts do I have?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["message"], "You have 1 account.");
    assert_eq!(json["model"], "gpt-4o-mini");

    let recorded_requests = recorded_requests.lock().await;
    assert_eq!(recorded_requests.len(), 2);

    let second_request_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second OpenAI request should include messages");
    assert!(second_request_messages.iter().any(|message| {
        message["role"] == "assistant"
            && message["tool_calls"][0]["id"] == "call_1"
            && message["tool_calls"][0]["function"]["name"] == "list_accounts"
    }));
    assert!(second_request_messages.iter().any(|message| {
        message["role"] == "tool"
            && message["tool_call_id"] == "call_1"
            && message["content"]
                .as_str()
                .expect("tool message should be serialized as JSON text")
                .contains("\"count\":1")
    }));
}

#[tokio::test]
async fn assistant_chat_returns_too_many_requests_when_semaphore_is_exhausted() {
    use tokio::sync::Semaphore;

    let pool = test_pool().await;
    let exhausted_semaphore = std::sync::Arc::new(Semaphore::new(0));
    let app = build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus::available())),
        asset_price_refresh_config: no_price_config(),
        http_client: reqwest::Client::new(),
        openai_api_key: None,
        assistant_models: new_shared_assistant_model_registry(None),
        assistant_chat_semaphore: exhausted_semaphore,
        openai_chat_url: crate::assistant::openai_chat_url().to_string(),
        openai_models_url: crate::assistant::openai_models_url().to_string(),
    });
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({ "messages": [{ "role": "user", "content": "hello" }] }),
    )
    .await;

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(json["error"]
        .as_str()
        .unwrap_or("")
        .contains("too many concurrent"));
}

// ── currencies ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn lists_allowed_currencies() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(app, "{ currencies }").await;

    assert_eq!(
        json["data"]["currencies"],
        json!(["CHF", "EUR", "GBP", "USD"])
    );
}

// ── assets ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn lists_assets() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        "{ assets { id symbol name assetType quoteSymbol isin currentPrice currentPriceCurrency currentPriceAsOf totalQuantity } }",
    )
    .await;

    let assets = &json["data"]["assets"];
    assert_eq!(assets[0]["symbol"], "AAPL");
    assert_eq!(assets[0]["name"], "Apple Inc.");
    assert_eq!(assets[0]["assetType"], "STOCK");
    assert!(assets[0]["quoteSymbol"].is_null());
    assert!(assets[0]["currentPrice"].is_null());
    assert!(assets[0]["totalQuantity"].is_null());
    assert!(assets[0]["avgCostBasis"].is_null());
    assert!(assets[0]["avgCostBasisCurrency"].is_null());
}

#[tokio::test]
async fn asset_avg_cost_basis_computed_from_buy_transactions() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::try_from("USD").unwrap(),
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::try_from("USD").unwrap(),
            amount: amt("3000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    // Buy 10 @ 100 and 10 @ 200 → avg = 150
    for (qty, price) in [("10", "100"), ("10", "200")] {
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: AssetTransactionType::Buy,
                trade_date: trade_date("2024-01-01"),
                quantity: AssetQuantity::try_from(qty).unwrap(),
                unit_price: AssetUnitPrice::try_from(price).unwrap(),
                currency_code: Currency::try_from("USD").unwrap(),
                notes: None,
            },
        )
        .await
        .expect("transaction insert should succeed");
    }

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ assets { avgCostBasis avgCostBasisCurrency } }").await;

    let asset = &json["data"]["assets"][0];
    assert_eq!(asset["avgCostBasis"], "150.000000");
    assert_eq!(asset["avgCostBasisCurrency"], "USD");
}

#[tokio::test]
async fn creates_asset_with_normalized_fields() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation {
            createAsset(input: {
                symbol: "  aapl  "
                name: "  Apple Inc.  "
                assetType: STOCK
                quoteSymbol: "  aapl  "
            }) { id symbol name assetType quoteSymbol }
        }"#,
    )
    .await;

    let asset = &json["data"]["createAsset"];
    assert_eq!(asset["symbol"], "AAPL");
    assert_eq!(asset["name"], "Apple Inc.");
    assert_eq!(asset["assetType"], "STOCK");
    assert_eq!(asset["quoteSymbol"], "AAPL");
}

#[tokio::test]
async fn creates_asset_and_fetches_price_immediately() {
    let server_url = start_test_quote_server(json!({
        "symbol": "AAPL",
        "price": "42.5",
        "currency": "USD"
    }))
    .await;

    let pool = test_pool().await;
    let config = AssetPriceRefreshConfig {
        coingecko_base_url: format!("{server_url}/quote"),
        ..no_price_config()
    };
    let app = build_app_with_price_config(pool, config);

    // Asset creation triggers a price refresh — we just verify the asset is created
    let json = gql(
        app,
        r#"mutation {
            createAsset(input: { symbol: "AAPL", name: "Apple Inc.", assetType: CRYPTO }) {
                id symbol
            }
        }"#,
    )
    .await;

    assert_eq!(json["data"]["createAsset"]["symbol"], "AAPL");
    assert!(json["errors"].is_null());
}

#[tokio::test]
async fn rejects_invalid_asset_creation_with_field_errors() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation { createAsset(input: { symbol: "   ", name: "   ", assetType: STOCK }) { id } }"#,
    )
    .await;

    assert!(json["data"]["createAsset"].is_null());
    let err = &json["errors"][0];
    assert_eq!(err["message"], "Asset validation failed");
    let field_errors = &err["extensions"]["field_errors"];
    assert!(field_errors["symbol"].is_array());
    assert!(field_errors["name"].is_array());
}

#[tokio::test]
async fn rejects_duplicate_asset_symbol() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        r#"mutation {
            createAsset(input: { symbol: "AAPL", name: "Apple Common Stock", assetType: STOCK, isin: "US0378331006" }) { id }
        }"#,
    )
    .await;

    let err = &json["errors"][0];
    assert_eq!(err["message"], "Asset validation failed");
    assert!(err["extensions"]["field_errors"]["symbol"].is_array());
}

#[tokio::test]
async fn rejects_duplicate_asset_isin() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: Some("US9229087690".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        r#"mutation {
            createAsset(input: { symbol: "VWCE", name: "Vanguard FTSE All-World UCITS ETF", assetType: ETF, isin: "US9229087690" }) { id }
        }"#,
    )
    .await;

    let err = &json["errors"][0];
    assert_eq!(err["message"], "Asset validation failed");
    assert!(err["extensions"]["field_errors"]["isin"].is_array());
}

#[tokio::test]
async fn gets_asset_detail() {
    let pool = test_pool().await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ asset(id: {}) {{ id symbol name assetType createdAt updatedAt }} }}",
            asset_id.as_i64()
        ),
    )
    .await;

    let asset = &json["data"]["asset"];
    assert_eq!(asset["symbol"], "AAPL");
    assert_eq!(asset["name"], "Apple Inc.");
    assert_eq!(asset["assetType"], "STOCK");
    assert!(asset["createdAt"].is_string());
}

#[tokio::test]
async fn updates_asset() {
    let pool = test_pool().await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ updateAsset(id: {}, input: {{ symbol: "AAPL", name: "Apple Inc. Updated", assetType: STOCK }}) {{ symbol name }} }}"#,
            asset_id.as_i64()
        ),
    )
    .await;

    assert_eq!(json["data"]["updateAsset"]["name"], "Apple Inc. Updated");
}

#[tokio::test]
async fn deletes_asset() {
    let pool = test_pool().await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!("mutation {{ deleteAsset(id: {}) }}", asset_id.as_i64()),
    )
    .await;

    assert_eq!(json["data"]["deleteAsset"], asset_id.as_i64());
}

#[tokio::test]
async fn rejects_deleting_asset_with_transactions() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("100.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("1").unwrap(),
            unit_price: AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!("mutation {{ deleteAsset(id: {}) }}", asset_id.as_i64()),
    )
    .await;

    assert!(json["data"]["deleteAsset"].is_null());
    assert_eq!(
        json["errors"][0]["message"],
        "Asset has transactions and cannot be deleted"
    );
}

// ── fx rates ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn lists_fx_rates_for_eur() {
    let pool = test_pool().await;

    for (from_currency, rate) in [(Currency::Usd, "0.920000"), (Currency::Gbp, "1.170000")] {
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

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        "{ fxRates { targetCurrency rates { currency rate } lastUpdated refreshStatus refreshError } }",
    )
    .await;

    let fx = &json["data"]["fxRates"];
    assert_eq!(fx["targetCurrency"], "EUR");
    assert_eq!(fx["refreshStatus"], "AVAILABLE");
    assert!(fx["refreshError"].is_null());
    let rates = fx["rates"].as_array().unwrap();
    assert!(
        rates
            .iter()
            .any(|r| r["currency"] == "USD" && r["rate"] == "0.92")
    );
    assert!(
        rates
            .iter()
            .any(|r| r["currency"] == "GBP" && r["rate"] == "1.17")
    );
}

#[tokio::test]
async fn returns_empty_fx_rates_when_no_eur_rates_exist() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ fxRates { rates { currency } } }").await;
    assert_eq!(json["data"]["fxRates"]["rates"], json!([]));
}

// ── portfolio ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn returns_portfolio_summary() {
    let pool = test_pool().await;

    let ibkr_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    for (from_currency, rate) in [(Currency::Usd, "0.920000"), (Currency::Gbp, "1.170000")] {
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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id: ibkr_id,
            currency: Currency::Usd,
            amount: amt("100.00"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        "{ portfolio { displayCurrency totalValueStatus totalValueAmount fxRefreshStatus } }",
    )
    .await;

    let p = &json["data"]["portfolio"];
    assert_eq!(p["displayCurrency"], "EUR");
    assert_eq!(p["totalValueStatus"], "OK");
    assert_eq!(p["totalValueAmount"], "92.000000");
    assert_eq!(p["fxRefreshStatus"], "AVAILABLE");
}

#[tokio::test]
async fn returns_empty_portfolio_summary_when_no_cash_exists() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        "{ portfolio { totalValueStatus totalValueAmount accountTotals { id } } }",
    )
    .await;

    let p = &json["data"]["portfolio"];
    assert_eq!(p["totalValueStatus"], "OK");
    assert_eq!(p["accountTotals"], json!([]));
}

#[tokio::test]
async fn returns_conversion_unavailable_portfolio_when_fx_is_missing() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
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
            amount: amt("100.00"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ portfolio { totalValueStatus totalValueAmount } }").await;

    let p = &json["data"]["portfolio"];
    assert_eq!(p["totalValueStatus"], "CONVERSION_UNAVAILABLE");
    assert!(p["totalValueAmount"].is_null());
}

// ── transactions ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn creates_asset_transaction() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("2000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{
                createTransaction(input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2026-03-20"
                    quantity: "10"
                    unitPrice: "150.00"
                    currencyCode: "USD"
                }}) {{ id accountId assetId transactionType tradeDate quantity unitPrice currencyCode }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let tx = &json["data"]["createTransaction"];
    assert_eq!(tx["transactionType"], "BUY");
    assert_eq!(tx["tradeDate"], "2026-03-20");
    assert_eq!(tx["quantity"], "10.000000");
    assert_eq!(tx["unitPrice"], "150.000000");
    assert_eq!(tx["currencyCode"], "USD");
}

#[tokio::test]
async fn lists_asset_transactions_in_trade_date_order() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("2000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    for (date, qty) in [
        ("2026-03-01", "5"),
        ("2026-01-15", "10"),
        ("2026-02-20", "3"),
    ] {
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: AssetTransactionType::Buy,
                trade_date: trade_date(date),
                quantity: AssetQuantity::try_from(qty).unwrap(),
                unit_price: AssetUnitPrice::try_from("100").unwrap(),
                currency_code: Currency::Usd,
                notes: None,
            },
        )
        .await
        .expect("transaction insert should succeed");
    }

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ transactions(accountId: {}) {{ tradeDate }} }}",
            account_id.as_i64()
        ),
    )
    .await;

    let txs = json["data"]["transactions"].as_array().unwrap();
    assert_eq!(txs.len(), 3);
    // Should be in descending trade date order
    assert_eq!(txs[0]["tradeDate"], "2026-03-01");
    assert_eq!(txs[1]["tradeDate"], "2026-02-20");
    assert_eq!(txs[2]["tradeDate"], "2026-01-15");
}

#[tokio::test]
async fn lists_all_transactions_without_filter() {
    let pool = test_pool().await;

    for (account_name_str, currency) in [("IBKR", Currency::Usd), ("Bank", Currency::Eur)] {
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: account_name(account_name_str),
                account_type: AccountType::Broker,
                base_currency: currency,
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt("100.000000"),
            },
        )
        .await
        .expect("balance insert should succeed");

        let asset_id = create_asset(
            &pool,
            CreateAssetInput {
                symbol: asset_symbol(&format!("SYM{account_name_str}")),
                name: asset_name(&format!("Name {account_name_str}")),
                asset_type: AssetType::Stock,
                quote_symbol: None,
                isin: None,
            },
        )
        .await
        .expect("asset insert should succeed");

        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: AssetTransactionType::Buy,
                trade_date: trade_date("2026-03-20"),
                quantity: AssetQuantity::try_from("1").unwrap(),
                unit_price: AssetUnitPrice::try_from("100").unwrap(),
                currency_code: currency,
                notes: None,
            },
        )
        .await
        .expect("transaction insert should succeed");
    }

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ transactions { id } }").await;

    assert_eq!(json["data"]["transactions"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn updates_asset_transaction() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("4000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let tx = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("10").unwrap(),
            unit_price: AssetUnitPrice::try_from("150").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2026-03-20"
                    quantity: "20"
                    unitPrice: "200.00"
                    currencyCode: "USD"
                }}) {{ quantity unitPrice }}
            }}"#,
            tx.id,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let updated = &json["data"]["updateTransaction"];
    assert_eq!(updated["quantity"], "20.000000");
    assert_eq!(updated["unitPrice"], "200.000000");
}

#[tokio::test]
async fn deletes_asset_transaction() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("100.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let tx = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("1").unwrap(),
            unit_price: AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!("mutation {{ deleteTransaction(id: {}) }}", tx.id),
    )
    .await;

    assert_eq!(json["data"]["deleteTransaction"], tx.id);
}

#[tokio::test]
async fn returns_not_found_when_deleting_missing_transaction() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "mutation { deleteTransaction(id: 999) }").await;

    assert!(json["data"]["deleteTransaction"].is_null());
    assert_eq!(json["errors"][0]["message"], "Transaction not found");
}

#[tokio::test]
async fn gets_transaction_detail() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("1000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let tx = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("5").unwrap(),
            unit_price: AssetUnitPrice::try_from("200").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("test note".to_string()),
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ transaction(id: {}) {{ transactionType tradeDate quantity notes }} }}",
            tx.id
        ),
    )
    .await;

    let t = &json["data"]["transaction"];
    assert_eq!(t["transactionType"], "BUY");
    assert_eq!(t["tradeDate"], "2026-03-20");
    assert_eq!(t["quantity"], "5.000000");
    assert_eq!(t["notes"], "test note");
}

#[tokio::test]
async fn lists_active_positions() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("1000.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("10").unwrap(),
            unit_price: AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ accountPositions(accountId: {}) {{ accountId assetId quantity }} }}",
            account_id.as_i64()
        ),
    )
    .await;

    let positions = json["data"]["accountPositions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["quantity"], "10.000000");
}

// ── accounts ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn creates_account() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation {
            createAccount(input: { name: "IBKR", accountType: BROKER, baseCurrency: "USD" }) {
                id name accountType baseCurrency summaryStatus
            }
        }"#,
    )
    .await;

    let account = &json["data"]["createAccount"];
    assert_eq!(account["name"], "IBKR");
    assert_eq!(account["accountType"], "BROKER");
    assert_eq!(account["baseCurrency"], "USD");
    assert_eq!(account["summaryStatus"], "OK");
}

#[tokio::test]
async fn creates_crypto_account() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation {
            createAccount(input: { name: "Ledger", accountType: CRYPTO, baseCurrency: "EUR" }) {
                accountType baseCurrency
            }
        }"#,
    )
    .await;

    assert_eq!(json["data"]["createAccount"]["accountType"], "CRYPTO");
    assert_eq!(json["data"]["createAccount"]["baseCurrency"], "EUR");
}

#[tokio::test]
async fn rejects_invalid_account_type() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation { createAccount(input: { name: "Test", accountType: INVALID, baseCurrency: "EUR" }) { id } }"#,
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["createAccount"].is_null());
}

#[tokio::test]
async fn rejects_invalid_base_currency() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let json = gql(
        app,
        r#"mutation { createAccount(input: { name: "Test", accountType: BANK, baseCurrency: "XYZ" }) { id } }"#,
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["createAccount"].is_null());
}

#[tokio::test]
async fn updates_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ updateAccount(id: {}, input: {{ name: "Updated Name", accountType: BROKER, baseCurrency: "USD" }}) {{ name }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert_eq!(json["data"]["updateAccount"]["name"], "Updated Name");
}

#[tokio::test]
async fn deletes_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!("mutation {{ deleteAccount(id: {}) }}", account_id.as_i64()),
    )
    .await;

    assert_eq!(json["data"]["deleteAccount"], account_id.as_i64());
}

#[tokio::test]
async fn returns_not_found_when_deleting_missing_account() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "mutation { deleteAccount(id: 999) }").await;

    assert!(json["data"]["deleteAccount"].is_null());
    assert_eq!(json["errors"][0]["message"], "Account not found");
}

#[tokio::test]
async fn lists_account_summaries_with_totals() {
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

    // USD balance: kept as manual cash (not affected by the BUY below)
    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("20.000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    // FX rate must be set up before the transaction so the cash impact can be computed
    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.500000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    // EUR balance: covers the cost of the BUY (2 × 80 USD × 0.5 = 80 EUR); after BUY it becomes 0
    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("80.000000"),
        },
    )
    .await
    .expect("eur balance insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("2").unwrap(),
            unit_price: AssetUnitPrice::try_from("80").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: AssetUnitPrice::try_from("100").unwrap(),
            currency: Currency::Usd,
            as_of: "2026-03-22T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("asset price insert should succeed");

    let app = build_router(pool);
    let json = gql(
        app,
        "{ accounts { summaryStatus cashTotalAmount assetTotalAmount totalAmount totalCurrency } }",
    )
    .await;

    // EUR balance is 0 after BUY deduction; only USD (20 × 0.5 = 10 EUR) remains in cash
    let accounts = json["data"]["accounts"].as_array().unwrap();
    assert_eq!(accounts[0]["summaryStatus"], "OK");
    assert_eq!(accounts[0]["cashTotalAmount"], "10.000000");
    assert_eq!(accounts[0]["assetTotalAmount"], "100.000000");
    assert_eq!(accounts[0]["totalAmount"], "110.000000");
    assert_eq!(accounts[0]["totalCurrency"], "EUR");
}

#[tokio::test]
async fn returns_conversion_unavailable_when_fx_rate_missing() {
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
            amount: amt("12.300000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_router(pool);
    let json = gql(
        app,
        "{ accounts { summaryStatus cashTotalAmount totalAmount totalCurrency } }",
    )
    .await;

    let accounts = json["data"]["accounts"].as_array().unwrap();
    assert_eq!(accounts[0]["summaryStatus"], "CONVERSION_UNAVAILABLE");
    assert!(accounts[0]["cashTotalAmount"].is_null());
    assert!(accounts[0]["totalAmount"].is_null());
    assert!(accounts[0]["totalCurrency"].is_null());
}

#[tokio::test]
async fn rounds_converted_account_totals() {
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

    for currency in [Currency::Usd, Currency::Gbp] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt("1.000000"),
            },
        )
        .await
        .expect("balance insert should succeed");
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency: currency,
                to_currency: Currency::Eur,
                rate: fx_rate("0.333333"),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    let app = build_router(pool);
    let json = gql(
        app,
        "{ accounts { summaryStatus cashTotalAmount assetTotalAmount totalAmount } }",
    )
    .await;

    let accounts = json["data"]["accounts"].as_array().unwrap();
    assert_eq!(accounts[0]["summaryStatus"], "OK");
    assert_eq!(accounts[0]["cashTotalAmount"], "0.666666");
    assert_eq!(accounts[0]["assetTotalAmount"], "0.000000");
    assert_eq!(accounts[0]["totalAmount"], "0.666666");
}

#[tokio::test]
async fn gets_account_detail_with_balances() {
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
            amount: amt("12.3"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_router(pool);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ name summaryStatus cashTotalAmount assetTotalAmount totalAmount totalCurrency balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;

    let a = &json["data"]["account"];
    assert_eq!(a["name"], "IBKR");
    assert_eq!(a["summaryStatus"], "CONVERSION_UNAVAILABLE");
    assert!(a["cashTotalAmount"].is_null());
    assert_eq!(a["assetTotalAmount"], "0.000000");
    assert!(a["totalAmount"].is_null());
    assert!(a["totalCurrency"].is_null());
    assert_eq!(a["balances"][0]["currency"], "USD");
    assert_eq!(a["balances"][0]["amount"], "12.300000");
}

#[tokio::test]
async fn returns_not_found_for_missing_account_detail() {
    let pool = test_pool().await;
    let app = build_router(pool);

    let json = gql(app, "{ account(id: 999) { name } }").await;

    assert!(json["data"]["account"].is_null());
    assert_eq!(json["errors"][0]["message"], "Account not found");
}

// ── balances ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn lists_account_balances() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, amount) in [(Currency::Usd, "100.00"), (Currency::Eur, "50.00")] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(amount),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;

    let balances = json["data"]["account"]["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);
}

#[tokio::test]
async fn creates_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ upsertBalance(accountId: {}, input: {{ currency: "USD", amount: "1234.56" }}) {{ currency amount }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    let balance = &json["data"]["upsertBalance"];
    assert_eq!(balance["currency"], "USD");
    assert_eq!(balance["amount"], "1234.560000");
}

#[tokio::test]
async fn rejects_invalid_amount_for_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ upsertBalance(accountId: {}, input: {{ currency: "USD", amount: "not_a_number" }}) {{ currency }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["upsertBalance"].is_null());
}

#[tokio::test]
async fn rejects_invalid_currency_for_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ upsertBalance(accountId: {}, input: {{ currency: "XYZ", amount: "100" }}) {{ currency }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["upsertBalance"].is_null());
}

#[tokio::test]
async fn updates_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
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
            amount: amt("100.00"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ upsertBalance(accountId: {}, input: {{ currency: "USD", amount: "999.99" }}) {{ amount }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert_eq!(json["data"]["upsertBalance"]["amount"], "999.990000");
}

#[tokio::test]
async fn returns_not_found_when_writing_balance_for_missing_account() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        r#"mutation { upsertBalance(accountId: 999, input: { currency: "USD", amount: "100" }) { currency } }"#,
    )
    .await;

    assert!(json["data"]["upsertBalance"].is_null());
    assert_eq!(json["errors"][0]["message"], "Account not found");
}

#[tokio::test]
async fn deletes_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
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
            amount: amt("100.00"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ deleteBalance(accountId: {}, currency: "USD") }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert_eq!(json["data"]["deleteBalance"], true);
}

#[tokio::test]
async fn returns_not_found_when_deleting_missing_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ deleteBalance(accountId: {}, currency: "USD") }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["data"]["deleteBalance"].is_null());
    assert_eq!(json["errors"][0]["message"], "Balance not found");
}
