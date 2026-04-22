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

use super::{AppState, AssistantState, build_router, build_router_with_state};
use crate::{
    AccountName, AccountType, Amount, AssetName, AssetPriceRefreshConfig, AssetQuantity,
    AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice, CreateAccountInput,
    CreateAssetInput, CreateAssetTransactionInput, CreateCashMovementInput, Currency, FxRate,
    FxRefreshAvailability, FxRefreshStatus, TradeDate, UpsertAssetPriceInput, UpsertFxRateInput,
    assistant::new_assistant_chat_semaphore, assistant::new_shared_assistant_model_registry,
    assistant::refresh_assistant_model_registry, create_account, create_asset,
    create_asset_transaction, create_cash_movement, init_db, upsert_asset_price, upsert_fx_rate,
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

async fn seed_balance(
    pool: &sqlx::SqlitePool,
    account_id: crate::AccountId,
    currency: crate::Currency,
    amount: crate::Amount,
) {
    create_cash_movement(
        pool,
        CreateCashMovementInput {
            account_id,
            currency,
            amount,
            date: trade_date("2024-01-01"),
            notes: None,
        },
    )
    .await
    .expect("balance seed should succeed");
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
        yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
        yahoo_finance_enabled: false,
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
        fcsapi_base_url: "http://127.0.0.1:1".to_string(),
        fcsapi_api_key: None,
        itick_base_url: "http://127.0.0.1:1".to_string(),
        itick_api_key: None,
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
        config_markdown: String::new(),
        assistant: AssistantState {
            openai_api_key: None,
            models: new_shared_assistant_model_registry(None, None, None),
            chat_semaphore: new_assistant_chat_semaphore(),
            openai_responses_url: crate::assistant::openai_responses_url().to_string(),
            openai_models_url: crate::assistant::openai_models_url().to_string(),
            mcp_client: None,
        },
    })
}

fn build_app_with_price_config(pool: sqlx::SqlitePool, config: AssetPriceRefreshConfig) -> Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus::available())),
        asset_price_refresh_config: config,
        http_client: reqwest::Client::new(),
        config_markdown: String::new(),
        assistant: AssistantState {
            openai_api_key: None,
            models: new_shared_assistant_model_registry(None, None, None),
            chat_semaphore: new_assistant_chat_semaphore(),
            openai_responses_url: crate::assistant::openai_responses_url().to_string(),
            openai_models_url: crate::assistant::openai_models_url().to_string(),
            mcp_client: None,
        },
    })
}

fn build_app_with_openai(
    pool: sqlx::SqlitePool,
    api_key: Option<&str>,
    openai_responses_url: String,
    openai_models_url: String,
) -> Router {
    build_app_with_openai_registry(
        pool,
        api_key,
        openai_responses_url,
        openai_models_url,
        new_shared_assistant_model_registry(api_key, None, None),
    )
}

fn build_app_with_openai_registry(
    pool: sqlx::SqlitePool,
    api_key: Option<&str>,
    openai_responses_url: String,
    openai_models_url: String,
    assistant_models: crate::assistant::SharedAssistantModelRegistry,
) -> Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus::available())),
        asset_price_refresh_config: no_price_config(),
        http_client: reqwest::Client::new(),
        config_markdown: String::new(),
        assistant: AssistantState {
            openai_api_key: api_key.map(str::to_string),
            models: assistant_models,
            chat_semaphore: new_assistant_chat_semaphore(),
            openai_responses_url,
            openai_models_url,
            mcp_client: None,
        },
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

/// Post to an SSE-streaming chat endpoint and collect all events as parsed JSON objects.
/// Returns (HTTP status, events). For the 429 case the status is non-200 and events is empty.
async fn post_chat_sse(app: Router, body: Value) -> (StatusCode, Vec<Value>) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/assistant/chat")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    if !status.is_success() {
        return (status, vec![]);
    }

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap_or("");
    let events = text
        .split("\n\n")
        .flat_map(|chunk| chunk.lines())
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str(data).ok())
        .collect();

    (status, events)
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
        "/v1/responses",
        post(move || {
            let payload = payload.clone();
            async move {
                axum::response::Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap()
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
    format!("http://{address}/v1/responses")
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
        "/v1/responses",
        post(move |Json(body): Json<Value>| {
            let recorded_requests = Arc::clone(&recorded_requests);
            let request_count = Arc::clone(&request_count);

            async move {
                recorded_requests.lock().await.push(body);

                let sse_body = match request_count.fetch_add(1, Ordering::SeqCst) {
                    0 => {
                        let created = json!({"type": "response.created", "response": {"id": "resp_1"}});
                        let output_item_added = json!({"type": "response.output_item.added", "output_index": 0, "item": {"type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "list_accounts", "arguments": ""}});
                        let func_done = json!({"type": "response.function_call_arguments.done", "item_id": "fc_1", "output_index": 0, "arguments": "{}"});
                        let completed = json!({"type": "response.completed", "response": {"id": "resp_1"}});
                        format!(
                            "data: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                            created, output_item_added, func_done, completed
                        )
                    }
                    _ => {
                        let created = json!({"type": "response.created", "response": {"id": "resp_2"}});
                        let delta1 = json!({"type": "response.output_text.delta", "delta": "You have "});
                        let delta2 = json!({"type": "response.output_text.delta", "delta": "1 account."});
                        let completed = json!({"type": "response.completed", "response": {"id": "resp_2"}});
                        format!(
                            "data: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                            created, delta1, delta2, completed
                        )
                    }
                };

                axum::response::Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(axum::body::Body::from(sse_body))
                    .unwrap()
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
    format!("http://{address}/v1/responses")
}

async fn start_test_openai_reasoning_server(recorded_requests: Arc<Mutex<Vec<Value>>>) -> String {
    let app = Router::new().route(
        "/v1/responses",
        post(move |Json(body): Json<Value>| {
            let recorded_requests = Arc::clone(&recorded_requests);

            async move {
                recorded_requests.lock().await.push(body);

                let created = json!({"type": "response.created", "response": {"id": "resp_r1"}});
                let reasoning1 = json!({"type": "response.reasoning_summary_text.delta", "delta": "Let me think"});
                let reasoning2 = json!({"type": "response.reasoning_summary_text.delta", "delta": " about this."});
                let text1 = json!({"type": "response.output_text.delta", "delta": "The answer "});
                let text2 = json!({"type": "response.output_text.delta", "delta": "is 42."});
                let completed = json!({"type": "response.completed", "response": {"id": "resp_r1"}});
                let sse_body = format!(
                    "data: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                    created, reasoning1, reasoning2, text1, text2, completed
                );

                axum::response::Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(axum::body::Body::from(sse_body))
                    .unwrap()
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
    format!("http://{address}/v1/responses")
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

    seed_balance(&pool, account_id, Currency::Eur, amt("125.50")).await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let (status, events) = post_chat_sse(
        app,
        json!({
            "messages": [
                { "role": "user", "content": "What does my portfolio look like?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let text_event = events
        .iter()
        .find(|e| e["type"] == "text")
        .expect("should have a text event");
    let message = text_event["text"]
        .as_str()
        .expect("text event should have text");
    assert!(message.contains("125.5 EUR"));
    assert!(message.contains("1 account"));
}

#[tokio::test]
async fn assistant_chat_handles_empty_prompt_with_backend_status_summary() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let (status, events) = post_chat_sse(app, json!({ "messages": [] })).await;

    assert_eq!(status, StatusCode::OK);
    let text_event = events
        .iter()
        .find(|e| e["type"] == "text")
        .expect("should have a text event");
    let message = text_event["text"]
        .as_str()
        .expect("text event should have text");
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
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"), None, None);
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
        crate::assistant::openai_responses_url().to_string(),
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
    let openai_responses_url = start_test_openai_tool_server(Arc::clone(&recorded_requests)).await;
    let models_url = start_test_openai_models_server(json!({
        "data": [
            { "id": "gpt-4.1-mini" },
            { "id": "gpt-4o-mini" }
        ]
    }))
    .await;
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"), None, None);
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
        openai_responses_url,
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

    let (status, events) = post_chat_sse(
        app,
        json!({
            "messages": [
                { "role": "user", "content": "How many accounts do I have?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        events.iter().any(|e| e["type"] == "text_delta"),
        "should have a text_delta event"
    );

    let recorded_requests = recorded_requests.lock().await;
    assert_eq!(recorded_requests[0]["model"], "gpt-4.1-mini");
}

#[tokio::test]
async fn assistant_models_selection_is_persisted_and_restored() {
    let pool = test_pool().await;
    let models_url = start_test_openai_models_server(json!({
        "data": [
            { "id": "gpt-4.1-mini" },
            { "id": "gpt-4o-mini" }
        ]
    }))
    .await;
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"), None, None);
    refresh_assistant_model_registry(
        &assistant_models,
        &reqwest::Client::new(),
        Some("test-key"),
        &models_url,
    )
    .await
    .expect("assistant models should refresh");

    let app = build_app_with_openai_registry(
        pool.clone(),
        Some("test-key"),
        crate::assistant::openai_responses_url().to_string(),
        models_url.clone(),
        assistant_models,
    );
    let (status, json) = put_json(
        app,
        "/assistant/models/selected",
        json!({ "model": "gpt-4.1-mini" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["selected_model"], "gpt-4.1-mini");

    let persisted_model =
        crate::storage::settings::get_app_setting(&pool, crate::assistant::SETTING_SELECTED_MODEL)
            .await
            .expect("selected model setting should load");
    assert_eq!(persisted_model.as_deref(), Some("gpt-4.1-mini"));

    let restored_models =
        new_shared_assistant_model_registry(Some("test-key"), persisted_model.as_deref(), None);
    refresh_assistant_model_registry(
        &restored_models,
        &reqwest::Client::new(),
        Some("test-key"),
        &models_url,
    )
    .await
    .expect("assistant models should refresh");

    let app = build_app_with_openai_registry(
        pool,
        Some("test-key"),
        crate::assistant::openai_responses_url().to_string(),
        models_url,
        restored_models,
    );
    let (status, json) = get_json(app, "/assistant/models").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["selected_model"], "gpt-4.1-mini");
}

#[tokio::test]
async fn assistant_chat_surfaces_openai_failures_as_bad_gateway() {
    let pool = test_pool().await;
    let openai_responses_url = start_test_openai_error_server(
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
        openai_responses_url,
        crate::assistant::openai_models_url().to_string(),
    );
    let (status, events) = post_chat_sse(
        app,
        json!({
            "messages": [
                { "role": "user", "content": "What does my portfolio look like?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let error_event = events
        .iter()
        .find(|e| e["type"] == "error")
        .expect("should have an error event");
    let error = error_event["error"]
        .as_str()
        .expect("error event should have an error message");
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
    let openai_responses_url = start_test_openai_tool_server(Arc::clone(&recorded_requests)).await;
    let app = build_app_with_openai(
        pool,
        Some("test-key"),
        openai_responses_url,
        crate::assistant::openai_models_url().to_string(),
    );
    let (status, events) = post_chat_sse(
        app,
        json!({
            "messages": [
                { "role": "user", "content": "How many accounts do I have?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let tool_call_event = events
        .iter()
        .find(|e| e["type"] == "tool_call")
        .expect("should have a tool_call event");
    assert_eq!(tool_call_event["name"], "list_accounts");
    let tool_result_event = events
        .iter()
        .find(|e| e["type"] == "tool_result")
        .expect("should have a tool_result event");
    assert!(
        tool_result_event["result"]
            .to_string()
            .contains("\"count\":1")
    );
    let full_text: String = events
        .iter()
        .filter(|e| e["type"] == "text_delta")
        .filter_map(|e| e["delta"].as_str())
        .collect();
    assert_eq!(full_text, "You have 1 account.");

    let recorded_requests = recorded_requests.lock().await;
    assert_eq!(recorded_requests.len(), 2);

    assert!(recorded_requests[1].get("previous_response_id").is_none());
    let second_request_input = recorded_requests[1]["input"]
        .as_array()
        .expect("second OpenAI request should include input items");
    assert_eq!(
        second_request_input
            .first()
            .expect("second request should retain original user input"),
        &json!({ "role": "user", "content": "How many accounts do I have?" })
    );
    assert!(second_request_input.iter().any(|item| {
        item["type"] == "function_call"
            && item["call_id"] == "call_1"
            && item["name"] == "list_accounts"
            && item["arguments"] == "{}"
    }));
    assert!(second_request_input.iter().any(|item| {
        item["type"] == "function_call_output"
            && item["call_id"] == "call_1"
            && item["output"]
                .as_str()
                .expect("tool output should be serialized as JSON text")
                .contains("\"count\":1")
    }));
}

#[tokio::test]
async fn assistant_chat_streams_reasoning_and_text_for_reasoning_model() {
    let pool = test_pool().await;
    let recorded_requests = Arc::new(Mutex::new(Vec::new()));
    let openai_responses_url =
        start_test_openai_reasoning_server(Arc::clone(&recorded_requests)).await;
    let models_url = start_test_openai_models_server(json!({
        "data": [{ "id": "o4-mini" }]
    }))
    .await;
    let assistant_models = new_shared_assistant_model_registry(Some("test-key"), None, None);
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
        openai_responses_url,
        models_url,
        assistant_models,
    );

    let (status, _) = put_json(
        app.clone(),
        "/assistant/models/selected",
        json!({ "model": "o4-mini" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, events) = post_chat_sse(
        app,
        json!({
            "messages": [
                { "role": "user", "content": "What is the meaning of life?" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let full_reasoning: String = events
        .iter()
        .filter(|e| e["type"] == "reasoning_delta")
        .filter_map(|e| e["delta"].as_str())
        .collect();
    assert_eq!(full_reasoning, "Let me think about this.");

    let full_text: String = events
        .iter()
        .filter(|e| e["type"] == "text_delta")
        .filter_map(|e| e["delta"].as_str())
        .collect();
    assert_eq!(full_text, "The answer is 42.");

    let recorded_requests = recorded_requests.lock().await;
    assert_eq!(recorded_requests.len(), 1);
    assert_eq!(recorded_requests[0]["reasoning"]["effort"], "medium");
    assert_eq!(recorded_requests[0]["reasoning"]["summary"], "detailed");
    assert_eq!(recorded_requests[0]["model"], "o4-mini");
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
        config_markdown: String::new(),
        assistant: AssistantState {
            openai_api_key: None,
            models: new_shared_assistant_model_registry(None, None, None),
            chat_semaphore: exhausted_semaphore,
            openai_responses_url: crate::assistant::openai_responses_url().to_string(),
            openai_models_url: crate::assistant::openai_models_url().to_string(),
            mcp_client: None,
        },
    });
    let (status, json) = post_json(
        app,
        "/assistant/chat",
        json!({ "messages": [{ "role": "user", "content": "hello" }] }),
    )
    .await;

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(
        json["error"]
            .as_str()
            .unwrap_or("")
            .contains("too many concurrent")
    );
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
    assert!(assets[0]["convertedTotalValue"].is_null());
    assert!(assets[0]["convertedTotalValueCurrency"].is_null());
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

    seed_balance(
        &pool,
        account_id,
        Currency::try_from("USD").unwrap(),
        amt("3000.000000"),
    )
    .await;

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
async fn asset_avg_cost_basis_is_null_when_bought_in_multiple_currencies() {
    let pool = test_pool().await;

    // EUR-base account: buys AAPL once in USD and once in EUR → mixed currencies
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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

    seed_balance(&pool, account_id, Currency::Eur, amt("10000.000000")).await;

    // USD→EUR rate needed for the first buy (transaction currency USD, account base EUR)
    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::try_from("USD").unwrap(),
            to_currency: Currency::Eur,
            rate: FxRate::try_from("0.900000").unwrap(),
        },
    )
    .await
    .unwrap();

    // First buy in USD
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: AssetQuantity::try_from("10").unwrap(),
            unit_price: AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::try_from("USD").unwrap(),
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    // Second buy in EUR — mixed currencies: avg_cost_basis must be null
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-06-01"),
            quantity: AssetQuantity::try_from("5").unwrap(),
            unit_price: AssetUnitPrice::try_from("95").unwrap(),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ assets { avgCostBasis avgCostBasisCurrency } }").await;

    let asset = &json["data"]["assets"][0];
    assert!(asset["avgCostBasis"].is_null());
    assert!(asset["avgCostBasisCurrency"].is_null());
}

#[tokio::test]
async fn asset_query_returns_converted_total_value_in_eur() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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

    seed_balance(&pool, account_id, Currency::Eur, amt("1000.000000")).await;

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: AssetUnitPrice::try_from("120").unwrap(),
            currency: Currency::Usd,
            as_of: "2024-01-02T00:00:00Z".to_string(),
        },
    )
    .await
    .expect("price insert should succeed");

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
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
        "{ assets { symbol convertedTotalValue convertedTotalValueCurrency } }",
    )
    .await;

    let asset = &json["data"]["assets"][0];
    assert_eq!(asset["symbol"], "AAPL");
    assert_eq!(asset["convertedTotalValue"], "1080.000000");
    assert_eq!(asset["convertedTotalValueCurrency"], "EUR");
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

    seed_balance(&pool, account_id, Currency::Usd, amt("100.000000")).await;

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

    seed_balance(&pool, ibkr_id, Currency::Usd, amt("100.00")).await;

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
async fn returns_portfolio_gain_amounts() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
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

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: AssetUnitPrice::try_from("100.000000").unwrap(),
            currency: Currency::Usd,
            as_of: "2020-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .expect("previous price insert should succeed");

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: AssetUnitPrice::try_from("120.000000").unwrap(),
            currency: Currency::Usd,
            as_of: "2999-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .expect("current price insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: AssetQuantity::try_from("10.000000").unwrap(),
            unit_price: AssetUnitPrice::try_from("90.000000").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(app, "{ portfolio { gain24hAmount totalGainAmount } }").await;

    let p = &json["data"]["portfolio"];
    assert_eq!(p["gain24hAmount"], "180.000000");
    assert_eq!(p["totalGainAmount"], "270.000000");
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

    seed_balance(&pool, account_id, Currency::Usd, amt("100.00")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("2000.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("2000.000000")).await;

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

        seed_balance(&pool, account_id, currency, amt("100.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("4000.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("100.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

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
async fn rejects_base_currency_change() {
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
            r#"mutation {{ updateAccount(id: {}, input: {{ name: "IBKR", accountType: BROKER, baseCurrency: "EUR" }}) {{ id }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["updateAccount"].is_null());
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
    seed_balance(&pool, account_id, Currency::Usd, amt("20.000000")).await;

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
    seed_balance(&pool, account_id, Currency::Eur, amt("80.000000")).await;

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

    seed_balance(&pool, account_id, Currency::Usd, amt("12.300000")).await;

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
        seed_balance(&pool, account_id, currency, amt("1.000000")).await;
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

    seed_balance(&pool, account_id, Currency::Usd, amt("12.3")).await;

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
        seed_balance(&pool, account_id, currency, amt(amount)).await;
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
async fn creates_cash_movement() {
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
            r#"mutation {{ createCashMovement(accountId: {}, input: {{ currency: "USD", amount: "1234.56", date: "2025-01-15" }}) {{ currency amount date }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    let movement = &json["data"]["createCashMovement"];
    assert_eq!(movement["currency"], "USD");
    assert_eq!(movement["amount"], "1234.560000");
    assert_eq!(movement["date"], "2025-01-15");
}

#[tokio::test]
async fn rejects_invalid_amount_for_cash_movement() {
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
            r#"mutation {{ createCashMovement(accountId: {}, input: {{ currency: "USD", amount: "not_a_number", date: "2025-01-15" }}) {{ currency }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["createCashMovement"].is_null());
}

#[tokio::test]
async fn rejects_invalid_currency_for_cash_movement() {
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
            r#"mutation {{ createCashMovement(accountId: {}, input: {{ currency: "XYZ", amount: "100", date: "2025-01-15" }}) {{ currency }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    assert!(json["errors"][0]["message"].is_string());
    assert!(json["data"]["createCashMovement"].is_null());
}

#[tokio::test]
async fn cash_movements_accumulate() {
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

    seed_balance(&pool, account_id, Currency::Usd, amt("100.00")).await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            r#"mutation {{ createCashMovement(accountId: {}, input: {{ currency: "USD", amount: "50.00", date: "2025-01-15" }}) {{ amount }} }}"#,
            account_id.as_i64()
        ),
    )
    .await;

    // Returns the movement amount, not the running balance
    assert_eq!(json["data"]["createCashMovement"]["amount"], "50.000000");
}

#[tokio::test]
async fn returns_not_found_when_creating_cash_movement_for_missing_account() {
    let pool = test_pool().await;
    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        r#"mutation { createCashMovement(accountId: 999, input: { currency: "USD", amount: "100", date: "2025-01-15" }) { currency } }"#,
    )
    .await;

    assert!(json["data"]["createCashMovement"].is_null());
    assert_eq!(json["errors"][0]["message"], "Account not found");
}

// ── FX-rate drift correctness tests ──────────────────────────────────────────
//
// These tests expose a correctness bug: when a transaction is deleted or
// updated after the FX rate between the transaction currency and the account
// base currency has changed, the cash-impact reversal uses the *current* live
// FX rate instead of the rate that was locked in at trade execution time.
//
// This means deleting a week-old cross-currency trade silently corrupts the
// account cash balance by the full FX-drift amount times the trade notional.
//
// Tests whose name starts with "bug_" assert the CORRECT outcome and are
// expected to FAIL until the issue is fixed. Tests whose name starts with
// "sanity_" verify scenarios the bug does not affect and must continue to PASS.
//
// The fix: store the FX rate on asset_transactions at creation time and use
// that stored rate—never the live fx_rates table—for reversals.

// ── shared setup helpers ──────────────────────────────────────────────────────

async fn setup_eur_broker_with_usd_asset(
    pool: &sqlx::SqlitePool,
    initial_eur: &str,
    usd_to_eur: &str,
) -> (crate::AccountId, crate::AssetId) {
    let account_id = create_account(
        pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account should be created");

    seed_balance(pool, account_id, Currency::Eur, amt(initial_eur)).await;

    upsert_fx_rate(
        pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate(usd_to_eur),
        },
    )
    .await
    .expect("fx rate should be set");

    let asset_id = create_asset(
        pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    (account_id, asset_id)
}

async fn buy_usd(
    pool: &sqlx::SqlitePool,
    account_id: crate::AccountId,
    asset_id: crate::AssetId,
    qty: &str,
    price: &str,
) -> i64 {
    create_asset_transaction(
        pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: AssetQuantity::try_from(qty).unwrap(),
            unit_price: AssetUnitPrice::try_from(price).unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy transaction should be created")
    .id
}

async fn sell_usd(
    pool: &sqlx::SqlitePool,
    account_id: crate::AccountId,
    asset_id: crate::AssetId,
    qty: &str,
    price: &str,
) -> i64 {
    create_asset_transaction(
        pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2024-01-02"),
            quantity: AssetQuantity::try_from(qty).unwrap(),
            unit_price: AssetUnitPrice::try_from(price).unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("sell transaction should be created")
    .id
}

async fn set_usd_to_eur(pool: &sqlx::SqlitePool, rate: &str) {
    upsert_fx_rate(
        pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate(rate),
        },
    )
    .await
    .expect("fx rate update should succeed");
}

async fn eur_balance_after_delete(pool: sqlx::SqlitePool, tx_id: i64, account_id: i64) -> String {
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!("mutation {{ deleteTransaction(id: {tx_id}) }}"),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!("{{ account(id: {account_id}) {{ balances {{ currency amount }} }} }}"),
    )
    .await;

    json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or("").to_string())
        .unwrap_or_default()
}

// ── GROUP 1: DELETE + FX drift ─────────────────────────────────────────────
// All eight tests below fail because the reversal uses the live FX rate.

#[tokio::test]
async fn bug_delete_buy_after_fx_rate_increase_returns_too_much_cash() {
    // Trade: BUY 10 @ 100 USD at rate 1.0 → cost 1000 EUR → balance: 1000 EUR
    // FX moves 1.0 → 1.5 before delete.
    // Bug: reversal credits 10*100*1.5 = 1500 EUR → balance becomes 2500 EUR.
    // Correct: reversal should credit exactly 1000 EUR → balance: 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn bug_delete_buy_after_fx_rate_decrease_returns_too_little_cash() {
    // Trade: BUY 10 @ 100 USD at rate 1.0 → cost 1000 EUR → balance: 1000 EUR
    // FX moves 1.0 → 0.5 before delete.
    // Bug: reversal credits 10*100*0.5 = 500 EUR → balance becomes 1500 EUR.
    // Correct: reversal should credit 1000 EUR → balance: 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "0.500000").await;

    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn bug_delete_buy_after_fx_rate_doubles_reversal_amount_is_doubled() {
    // Trade: BUY 10 @ 100 USD at rate 0.500000 → cost 500 EUR → balance: 1500 EUR
    // FX doubles: 0.5 → 1.0 before delete.
    // Bug: reversal credits 10*100*1.0 = 1000 EUR → balance becomes 2500 EUR.
    // Correct: reversal should credit 500 EUR → balance: 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "0.500000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.000000").await;

    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn bug_delete_buy_after_large_fx_swing_produces_balance_above_initial_deposit() {
    // Initial deposit: 1000 EUR.
    // Trade: BUY 10 @ 100 USD at rate 0.500000 → cost 500 EUR → balance: 500 EUR.
    // FX moves to 2.0 (4x original rate) before delete.
    // Bug: reversal credits 10*100*2.0 = 2000 EUR → balance 2500 EUR > 1000 EUR initial.
    // Correct: balance must never exceed initial deposit after a round-trip.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "1000.000000", "0.500000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "2.000000").await;

    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    // Balance must equal initial deposit: the round-trip should be a no-op.
    assert_eq!(balance, "1000.000000");
}

#[tokio::test]
async fn bug_delete_sell_after_fx_rate_increase_deducts_too_much_cash() {
    // Setup: BUY 10 @ 100 USD at 1.0 → balance: 2000 EUR.
    //        SELL 10 @ 100 USD at 1.0 → gain 1000 EUR → balance: 3000 EUR.
    // FX moves 1.0 → 1.5 before the SELL is deleted.
    // Bug: reversing the SELL = applying a BUY at 1.5 → costs 1500 EUR → balance: 1500 EUR.
    // Correct: reversing the SELL should deduct exactly 1000 EUR → balance: 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "3000.000000", "1.000000").await;
    buy_usd(&pool, account_id, asset_id, "10", "100").await;
    let sell_id = sell_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let balance = eur_balance_after_delete(pool, sell_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn bug_delete_sell_after_fx_rate_decrease_deducts_too_little_cash() {
    // Setup: BUY 10 @ 100 USD at 1.0 → balance: 2000 EUR.
    //        SELL 10 @ 100 USD at 1.0 → balance: 3000 EUR.
    // FX moves 1.0 → 0.5 before the SELL is deleted.
    // Bug: reversing at 0.5 deducts only 500 EUR → balance becomes 2500 EUR.
    // Correct: should deduct 1000 EUR → balance: 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "3000.000000", "1.000000").await;
    buy_usd(&pool, account_id, asset_id, "10", "100").await;
    let sell_id = sell_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "0.500000").await;

    let balance = eur_balance_after_delete(pool, sell_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

// Note: a CHF-base account buying USD assets is not tested here because the
// system only stores FX rates as X→EUR. USD→CHF is therefore an invalid pair
// and would be rejected at transaction creation time. The EUR-base tests
// already provide full coverage of the cross-currency delete bug.

#[tokio::test]
async fn bug_two_buys_delete_both_after_fx_change_errors_compound() {
    // Each deleted trade applies the drift independently, so n deletes = n×error.
    // BUY 10 @ 100 USD at 1.0 → −1000 EUR → balance: 2000 EUR
    // BUY  5 @ 100 USD at 1.0 → −500  EUR → balance: 1500 EUR
    // FX moves to 2.0.
    // Delete first:  reversal = 2000 EUR → balance: 3500 EUR  (should be 2500)
    // Delete second: reversal = 1000 EUR → balance: 4500 EUR  (should be 3000)
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "3000.000000", "1.000000").await;
    let tx1 = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    let tx2 = buy_usd(&pool, account_id, asset_id, "5", "100").await;
    set_usd_to_eur(&pool, "2.000000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(app, &format!("mutation {{ deleteTransaction(id: {tx1}) }}")).await;
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(app, &format!("mutation {{ deleteTransaction(id: {tx2}) }}")).await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(balance, "3000.000000");
}

// ── GROUP 2: DELETE sanity (must continue to PASS) ─────────────────────────

#[tokio::test]
async fn sanity_delete_same_currency_tx_always_restores_balance_correctly() {
    // No FX conversion: USD account, USD transaction. The bug cannot appear.
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("USD Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("2000.000000")).await;

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
    .unwrap();

    // BUY in the same currency as the account — no FX lookup needed.
    let tx_id = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: AssetQuantity::try_from("10").unwrap(),
            unit_price: AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap()
    .id;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!("mutation {{ deleteTransaction(id: {tx_id}) }}"),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "USD")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn sanity_delete_cross_currency_tx_at_unchanged_fx_rate_restores_balance_correctly() {
    // When the FX rate does not change between trade and delete, the reversal
    // is exact and the balance is fully restored.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    // No rate change between create and delete.
    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

// ── GROUP 3: UPDATE + FX drift ─────────────────────────────────────────────
// Updating a transaction reverses the old cash impact and applies the new one,
// both at the *current* FX rate. When the rate has moved, the old reversal
// uses the wrong rate, corrupting the balance.

#[tokio::test]
async fn bug_update_quantity_decrease_after_fx_rate_increase_refunds_wrong_amount() {
    // Trade: BUY 10 @ 100 USD at 1.0 → cost 1000 EUR → balance: 1000 EUR.
    // FX moves to 1.5.
    // Update: reduce quantity 10 → 5 (same price, same currency).
    //   Reverse at 1.5: credit 10*100*1.5 = 1500 EUR → balance: 2500 EUR.
    //   Apply at 1.5:  debit  5*100*1.5 = 750  EUR → balance: 1750 EUR.
    // Correct (using original rate):
    //   Reverse at 1.0: credit 1000 EUR → balance: 2000 EUR.
    //   Apply at 1.0:  debit  500  EUR → balance: 1500 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "5"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(balance, "1500.000000");
}

#[tokio::test]
async fn bug_update_quantity_increase_after_fx_rate_decrease_deducts_wrong_amount() {
    // Trade: BUY 10 @ 100 USD at 1.0 → cost 1000 EUR → balance: 2000 EUR.
    // FX moves to 0.5.
    // Update: increase quantity 10 → 20 (same price).
    //   Reverse at 0.5: credit 10*100*0.5 = 500 EUR → balance: 2500 EUR.
    //   Apply at 0.5:  debit 20*100*0.5 = 1000 EUR → balance: 1500 EUR.
    // Correct (same-currency update uses stored rate for both operations):
    //   Reverse at stored 1.0: credit 1000 EUR → balance: 3000 EUR.
    //   Apply at stored 1.0:  debit  2000 EUR → balance: 1000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "3000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "0.500000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "20"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    // Correct: reversal at stored 1.0 = +1000 EUR; re-apply at stored 1.0 = −2000 EUR → balance: 1000 EUR.
    assert_eq!(balance, "1000.000000");
}

#[tokio::test]
async fn bug_update_unit_price_after_fx_rate_change_corrupts_balance() {
    // Trade: BUY 10 @ 100 USD at 1.0 → cost 1000 EUR → balance: 1000 EUR.
    // FX moves to 2.0.
    // Update: change price 100 → 50 (same quantity).
    //   Reverse at 2.0: credit 10*100*2.0 = 2000 EUR → balance: 3000 EUR.
    //   Apply at 2.0:  debit  10*50*2.0  = 1000 EUR → balance: 2000 EUR.
    // Correct (same-currency update uses stored rate for both operations):
    //   Reverse at stored 1.0: credit 1000 EUR → balance: 2000 EUR.
    //   Apply at stored 1.0:  debit    500 EUR → balance: 1500 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "2.000000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "10"
                    unitPrice: "50.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    // Correct: reversal at stored 1.0 = +1000 EUR; re-apply at stored 1.0 = −500 EUR → balance: 1500 EUR.
    assert_eq!(balance, "1500.000000");
}

#[tokio::test]
async fn sanity_update_trade_date_only_after_fx_drift_does_not_corrupt_balance() {
    // Changing only the trade date does not touch price, qty, or currency.
    // The reverse and re-apply use the same values at the same live rate,
    // so they cancel exactly — the balance is unaffected even after FX drift.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-06-01"
                    quantity: "10"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    // Trade date is not a cash field; balance should be unchanged at 1000 EUR.
    assert_eq!(balance, "1000.000000");
}

#[tokio::test]
async fn bug_update_currency_from_usd_to_eur_after_fx_drift_corrupts_balance() {
    // Trade: BUY 10 @ 100 USD at 1.0 → cost 1000 EUR → balance: 1000 EUR.
    // FX moves to 1.5.
    // Update: change transaction currency USD → EUR (keep price 100).
    //   Reverse USD BUY at 1.5: credit 10*100*1.5 = 1500 EUR → balance: 2500 EUR.
    //   Apply EUR BUY at 1.0:  debit  10*100*1.0 = 1000 EUR → balance: 1500 EUR.
    // Correct: reverse at original rate 1.0 gives back 1000 EUR;
    //          apply 1000 EUR cost → net zero → balance stays at 1000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "10"
                    unitPrice: "100.00"
                    currencyCode: "EUR"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    // The cost in EUR is the same (100 EUR/share × 10), so balance should not change.
    assert_eq!(balance, "1000.000000");
}

// ── GROUP 4: UPDATE sanity (must continue to PASS) ─────────────────────────

#[tokio::test]
async fn sanity_update_notes_only_after_fx_change_does_not_corrupt_balance() {
    // When only the notes field changes, the reverse and re-apply use identical
    // price/qty/currency at the same (current) rate, so they cancel exactly.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "10"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                    notes: "corrected note"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(balance, "1000.000000");
}

#[tokio::test]
async fn sanity_update_with_identical_values_after_fx_change_does_not_corrupt_balance() {
    // Submitting the exact same payload as the original trade (no actual edit)
    // should be a cash no-op even after FX drift, because reverse and re-apply
    // are symmetric at the same live rate.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "2.000000").await;

    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "10"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(balance, "1000.000000");
}

// ── GROUP 5: Compound scenarios ────────────────────────────────────────────

#[tokio::test]
async fn bug_update_then_delete_after_fx_drift_compounds_the_balance_error() {
    // Each operation (update, then delete) independently uses the live FX rate
    // for its reversal, compounding the error from the original trade rate.
    //
    // Trade: BUY 10 @ 100 USD at 1.0 → cost 1000 EUR → balance: 1000 EUR.
    // FX moves to 1.5.
    //
    // Step 1 — Update (qty 10 → 5):
    //   Reverse at 1.5: +1500 → 2500 EUR.
    //   Apply at 1.5:   −750  → 1750 EUR.  (correct would be 1500 EUR)
    //
    // Step 2 — Delete the updated tx (BUY 5 @ 100):
    //   Reverse at 1.5: +750 → 2500 EUR.   (correct would be 2000 EUR)
    //
    // Final balance 2500 EUR ≠ initial 2000 EUR.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "2000.000000", "1.000000").await;
    let tx_id = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    set_usd_to_eur(&pool, "1.500000").await;

    // Update qty 10 → 5
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(
        app,
        &format!(
            r#"mutation {{
                updateTransaction(id: {tx_id}, input: {{
                    accountId: {}
                    assetId: {}
                    transactionType: BUY
                    tradeDate: "2024-01-01"
                    quantity: "5"
                    unitPrice: "100.00"
                    currencyCode: "USD"
                }}) {{ id }}
            }}"#,
            account_id.as_i64(),
            asset_id.as_i64()
        ),
    )
    .await;

    // Delete the updated transaction
    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;
    assert_eq!(balance, "2000.000000");
}

#[tokio::test]
async fn bug_create_delete_round_trip_is_not_idempotent_when_fx_changes() {
    // A create followed by a delete should always be a net no-op regardless of
    // what happens to the FX rate in between.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "5000.000000", "0.800000").await;

    // Create BUY: 20 * 200 * 0.8 = 3200 EUR cost → balance: 1800 EUR.
    let tx_id = buy_usd(&pool, account_id, asset_id, "20", "200").await;

    // Simulate a week passing: FX moves significantly.
    set_usd_to_eur(&pool, "1.200000").await;

    // Delete the trade.
    let balance = eur_balance_after_delete(pool, tx_id, account_id.as_i64()).await;

    // Balance must equal the initial deposit — the round-trip is a no-op.
    assert_eq!(balance, "5000.000000");
}

#[tokio::test]
async fn bug_sequential_fx_changes_each_delete_uses_different_wrong_rate() {
    // Three trades created at the same original rate; FX drifts differently
    // before each deletion. Each reversal uses the rate at deletion time,
    // not the original trade rate.
    let pool = test_pool().await;
    let (account_id, asset_id) =
        setup_eur_broker_with_usd_asset(&pool, "6000.000000", "1.000000").await;

    // Three identical BUYs: each costs 1000 EUR → balance after all: 3000 EUR.
    let tx1 = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    let tx2 = buy_usd(&pool, account_id, asset_id, "10", "100").await;
    let tx3 = buy_usd(&pool, account_id, asset_id, "10", "100").await;

    // Delete tx1 with rate 1.1 → reversal = 1100 EUR (should be 1000)
    set_usd_to_eur(&pool, "1.100000").await;
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(app, &format!("mutation {{ deleteTransaction(id: {tx1}) }}")).await;

    // Delete tx2 with rate 1.5 → reversal = 1500 EUR (should be 1000)
    set_usd_to_eur(&pool, "1.500000").await;
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(app, &format!("mutation {{ deleteTransaction(id: {tx2}) }}")).await;

    // Delete tx3 with rate 0.5 → reversal = 500 EUR (should be 1000)
    set_usd_to_eur(&pool, "0.500000").await;
    let app = build_app_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    gql(app, &format!("mutation {{ deleteTransaction(id: {tx3}) }}")).await;

    let app = build_app_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let json = gql(
        app,
        &format!(
            "{{ account(id: {}) {{ balances {{ currency amount }} }} }}",
            account_id.as_i64()
        ),
    )
    .await;
    let balance = json["data"]["account"]["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["currency"] == "EUR")
        .map(|b| b["amount"].as_str().unwrap_or(""))
        .unwrap_or("");
    // All three trades reversed → balance should equal initial 6000 EUR.
    // Bug: reversals at 1.1, 1.5, 0.5 give 1100+1500+500=3100 surplus over the
    //      3000 EUR already remaining → balance = 3000+3100 = 6100 EUR.
    assert_eq!(balance, "6000.000000");
}
