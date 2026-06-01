//! Integration tests for `tyche-api`.
//!
//! Drive the fully-layered router via `tower::ServiceExt::oneshot` — no socket
//! bound, no global metrics recorder installed. Covers the happy paths, the
//! end-to-end simulate → attest → verify loop, and the M1H resilience surfaces
//! (rate-limit 429, body-limit 413).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use tyche_api::build_app;
use tyche_api::rate_limit::RateLimiter;
use tyche_api::state::AppState;

fn state_with_limiter(limiter: RateLimiter) -> AppState {
    AppState::new(tyche_api::metrics::test_handle(), Arc::new(limiter))
}

fn default_state() -> AppState {
    // Generous limiter so non-rate-limit tests never trip it.
    state_with_limiter(RateLimiter::new(10_000.0, 10_000.0))
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn post_json(uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
}

fn sample_portfolio() -> Value {
    let raw = include_str!("../../../examples/synthetic_portfolio/portfolio.json");
    serde_json::from_str(raw).unwrap()
}

fn sample_scenario() -> Value {
    let raw = include_str!("../../../examples/synthetic_portfolio/scenarios.json");
    let scenarios: Value = serde_json::from_str(raw).unwrap();
    // scenarios.json is an array; take the first.
    scenarios.as_array().unwrap()[0].clone()
}

#[tokio::test]
async fn healthz_ok() {
    let resp = build_app(default_state())
        .oneshot(get("/healthz"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_ok() {
    let resp = build_app(default_state())
        .oneshot(get("/readyz"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn version_has_model_version() {
    let resp = build_app(default_state())
        .oneshot(get("/version"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v.get("model_version").is_some());
    assert!(v.get("commit").is_some());
}

#[tokio::test]
async fn metrics_endpoint_renders() {
    let resp = build_app(default_state())
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn simulate_returns_metrics() {
    let req_body = json!({
        "portfolio": sample_portfolio(),
        "scenario": sample_scenario(),
        "config": { "n_paths": 2000, "chunk_size": 500 }
    });
    let resp = build_app(default_state())
        .oneshot(post_json("/v1/simulate", &req_body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v.get("expected_loss").is_some());
    assert!(v.get("var_99").is_some());
    assert_eq!(v.get("n_paths").unwrap().as_u64().unwrap(), 2000);
}

#[tokio::test]
async fn simulate_rejects_garbage() {
    let resp = build_app(default_state())
        .oneshot(post_json(
            "/v1/simulate",
            &json!({"portfolio": "not a portfolio"}),
        ))
        .await
        .unwrap();
    // Malformed JSON body → 422 from the extractor, or 400 from validation.
    assert!(
        resp.status() == StatusCode::UNPROCESSABLE_ENTITY
            || resp.status() == StatusCode::BAD_REQUEST,
        "got {}",
        resp.status()
    );
}

#[tokio::test]
async fn end_to_end_simulate_attest_verify() {
    let app = build_app(default_state());

    // 1. simulate
    let sim_resp = app
        .clone()
        .oneshot(post_json(
            "/v1/simulate",
            &json!({
                "portfolio": sample_portfolio(),
                "scenario": sample_scenario(),
                "config": { "n_paths": 1000, "chunk_size": 500 }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(sim_resp.status(), StatusCode::OK);
    let metrics = body_json(sim_resp).await;

    // 2. attest (ephemeral key minted server-side)
    let attest_resp = app
        .clone()
        .oneshot(post_json(
            "/v1/attest",
            &json!({
                "portfolio": sample_portfolio(),
                "scenario": sample_scenario(),
                "metrics": metrics,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(attest_resp.status(), StatusCode::OK);
    let attest = body_json(attest_resp).await;
    let record = attest.get("record").unwrap().clone();
    assert!(attest.get("ephemeral_signer_sk_hex").unwrap().is_string());

    // 3. verify — the freshly-built record must verify
    let verify_resp = app
        .oneshot(post_json("/v1/verify", &json!({ "record": record })))
        .await
        .unwrap();
    assert_eq!(verify_resp.status(), StatusCode::OK);
    let verify = body_json(verify_resp).await;
    assert!(verify.get("verified").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn verify_rejects_tampered_record() {
    let app = build_app(default_state());
    // Build a real record first.
    let metrics = {
        let r = app
            .clone()
            .oneshot(post_json(
                "/v1/simulate",
                &json!({"portfolio": sample_portfolio(), "scenario": sample_scenario(),
                        "config": {"n_paths": 500, "chunk_size": 500}}),
            ))
            .await
            .unwrap();
        body_json(r).await
    };
    let attest = {
        let r = app
            .clone()
            .oneshot(post_json(
                "/v1/attest",
                &json!({"portfolio": sample_portfolio(), "scenario": sample_scenario(), "metrics": metrics}),
            ))
            .await
            .unwrap();
        body_json(r).await
    };
    let mut record = attest.get("record").unwrap().clone();
    // Tamper: flip the input hash.
    record["input_hash"] = json!("0".repeat(64));

    let resp = app
        .oneshot(post_json("/v1/verify", &json!({ "record": record })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(!v.get("verified").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn rate_limit_returns_429_with_retry_after() {
    // capacity 2, no refill within the test window → 3rd request denied.
    let app = build_app(state_with_limiter(RateLimiter::new(2.0, 0.001)));

    let r1 = app.clone().oneshot(get("/healthz")).await.unwrap();
    assert_eq!(r1.status(), StatusCode::OK);
    let r2 = app.clone().oneshot(get("/healthz")).await.unwrap();
    assert_eq!(r2.status(), StatusCode::OK);
    let r3 = app.oneshot(get("/healthz")).await.unwrap();
    assert_eq!(r3.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(r3.headers().contains_key(header::RETRY_AFTER));
}

#[tokio::test]
async fn rate_limit_is_per_tenant() {
    let app = build_app(state_with_limiter(RateLimiter::new(1.0, 0.001)));

    // Tenant A burns its single token.
    let a1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-tyche-tenant", "firm-a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(a1.status(), StatusCode::OK);
    let a2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-tyche-tenant", "firm-a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(a2.status(), StatusCode::TOO_MANY_REQUESTS);

    // Tenant B is unaffected.
    let b1 = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-tyche-tenant", "firm-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(b1.status(), StatusCode::OK);
}

#[tokio::test]
async fn oversized_body_rejected() {
    // 2 MiB body exceeds the 1 MiB DefaultBodyLimit.
    let big = "x".repeat(2 * 1024 * 1024);
    let req = Request::builder()
        .method("POST")
        .uri("/v1/simulate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(big))
        .unwrap();
    let resp = build_app(default_state()).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
