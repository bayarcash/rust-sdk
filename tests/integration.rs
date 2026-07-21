//! HTTP-level integration tests driving the real [`Bayarcash`] client against a
//! `wiremock` mock server (via [`Bayarcash::set_base_url`]).
//!
//! Covers: request/response round-trips for the main operations, the v3-only
//! guards, and the full error-status mapping (422/404/400/429/other).

use bayarcash::{ApiVersion, Bayarcash, Error, PaymentIntentRequest, TransactionFilters};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer, version: ApiVersion) -> Bayarcash {
    Bayarcash::new("test_token")
        .set_api_version(version)
        .set_base_url(server.uri())
}

#[tokio::test]
async fn create_payment_intent_round_trip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payment-intents"))
        .and(header("authorization", "Bearer test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_123",
            "url": "https://pay.example/redirect",
            "order_number": "INV-1",
            "amount": "10.00",
            "status": "0",
            "payer_email": "a@b.com"
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let req = PaymentIntentRequest::new("portal", "INV-1", "10.00", "Ahmad", "a@b.com")
        .payment_channel(Bayarcash::FPX)
        .signed("secret");

    let intent = bc.create_payment_intent(&req).await.unwrap();
    assert_eq!(intent.id.as_deref(), Some("pi_123"));
    assert_eq!(intent.url.as_deref(), Some("https://pay.example/redirect"));
    assert_eq!(intent.amount, Some(10.0));
    assert_eq!(intent.status.as_deref(), Some("0"));
}

#[tokio::test]
async fn get_transaction_round_trip() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/transactions/trx_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "trx_1",
            "order_number": "INV-1",
            "status": 3,
            "amount": 10.0,
            "status_description": "Successful"
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let trx = bc.get_transaction("trx_1").await.unwrap();
    assert_eq!(trx.id.as_deref(), Some("trx_1"));
    // Integer status normalised to string, and the helper parses it back.
    assert_eq!(trx.status.as_deref(), Some("3"));
    assert_eq!(trx.status_code(), Some(3));
}

#[tokio::test]
async fn get_all_transactions_v3_with_meta() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/transactions"))
        .and(query_param("order_number", "INV-1"))
        .and(query_param("status", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ { "id": "trx_1", "order_number": "INV-1", "status": 3 } ],
            "meta": { "current_page": 1, "total": 1 }
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V3);
    let filters = TransactionFilters::new().order_number("INV-1").status("3");
    let list = bc.get_all_transactions(&filters).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id.as_deref(), Some("trx_1"));
    assert_eq!(list.meta["total"], serde_json::json!(1));
}

#[tokio::test]
async fn get_portals_and_channels() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/portals"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "portal_key": "pk_1", "portal_name": "Main", "payment_channels": [ {"id": 1, "name": "FPX"} ] },
                { "portal_key": "pk_2", "portal_name": "Other", "payment_channels": [] }
            ]
        })))
        .expect(2)
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let portals = bc.get_portals().await.unwrap();
    assert_eq!(portals.len(), 2);
    assert_eq!(portals[0].portal_key.as_deref(), Some("pk_1"));

    let channels = bc.get_channels("pk_1").await.unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0]["name"], serde_json::json!("FPX"));
}

#[tokio::test]
async fn fpx_banks_list_bare_array() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/banks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "bank_name": "Maybank", "bank_code": "MB2U0227", "bank_availability": 1 },
            { "bank_name": "CIMB", "bank_code": "BCBB0235", "bank_availability": true }
        ])))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let banks = bc.fpx_banks_list().await.unwrap();
    assert_eq!(banks.len(), 2);
    assert_eq!(banks[0].bank_name.as_deref(), Some("Maybank"));
    // availability tolerates int `1` and bool `true`.
    assert_eq!(banks[0].bank_availability, Some(true));
    assert_eq!(banks[1].bank_availability, Some(true));
}

#[tokio::test]
async fn cancel_payment_intent_v3() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/payment-intents/pi_9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_9",
            "status": "cancelled"
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V3);
    let intent = bc.cancel_payment_intent("pi_9").await.unwrap();
    assert_eq!(intent.id.as_deref(), Some("pi_9"));
    assert_eq!(intent.status.as_deref(), Some("cancelled"));
}

// --- v3-only guards ----------------------------------------------------------

#[tokio::test]
async fn v3_guard_rejects_on_v2() {
    let client = Bayarcash::new("token");
    assert!(matches!(
        client.get_all_transactions(&TransactionFilters::new()).await,
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        client.cancel_payment_intent("pi").await,
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        client.get_payment_intent("pi").await,
        Err(Error::Unsupported(_))
    ));
}

// --- error mapping -----------------------------------------------------------

#[tokio::test]
async fn maps_422_validation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payment-intents"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "message": "The given data was invalid.",
            "errors": { "amount": ["The amount must be at least 1.00."] }
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let req = PaymentIntentRequest::new("portal", "INV-1", "0.00", "Ahmad", "a@b.com");
    match bc.create_payment_intent(&req).await {
        Err(Error::Validation(v)) => {
            assert_eq!(v.message.as_deref(), Some("The given data was invalid."));
            assert_eq!(
                v.errors().get("amount").map(|e| e.as_slice()),
                Some(["The amount must be at least 1.00.".to_string()].as_slice())
            );
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[tokio::test]
async fn maps_404_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/transactions/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    assert!(matches!(
        bc.get_transaction("missing").await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn maps_400_failed_action_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payment-intents"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "message": "Portal is inactive"
        })))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    let req = PaymentIntentRequest::new("portal", "INV-1", "10.00", "Ahmad", "a@b.com");
    match bc.create_payment_intent(&req).await {
        Err(Error::FailedAction(msg)) => assert_eq!(msg, "Portal is inactive"),
        other => panic!("expected FailedAction, got {other:?}"),
    }
}

#[tokio::test]
async fn maps_429_rate_limit_with_reset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/transactions/trx"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-reset", "1700000000")
                .set_body_string("slow down"),
        )
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    match bc.get_transaction("trx").await {
        Err(Error::RateLimitExceeded { resets_at }) => {
            assert_eq!(resets_at, Some(1_700_000_000));
        }
        other => panic!("expected RateLimitExceeded, got {other:?}"),
    }
}

#[tokio::test]
async fn maps_other_status_to_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/transactions/trx"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let bc = client(&server, ApiVersion::V2);
    match bc.get_transaction("trx").await {
        Err(Error::Api { status, body }) => {
            assert_eq!(status, 500);
            assert_eq!(body, "boom");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}
