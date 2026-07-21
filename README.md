# Bayarcash Payment Gateway Rust SDK

[![Crates.io](https://img.shields.io/crates/v/bayarcash.svg)](https://crates.io/crates/bayarcash)
[![Downloads](https://img.shields.io/crates/d/bayarcash.svg)](https://crates.io/crates/bayarcash)
[![docs.rs](https://img.shields.io/docsrs/bayarcash)](https://docs.rs/bayarcash)
[![License](https://img.shields.io/crates/l/bayarcash.svg)](LICENSE)

An idiomatic, async Rust client for the [Bayarcash](https://bayarcash.com) Payment Gateway API — a feature-parity port of the official Bayarcash PHP SDK. It supports API **v2** (default) and **v3**, and is byte-compatible with the gateway for checksum generation and callback verification.

## Table of Contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Quick Start: Accept a Payment](#quick-start-accept-a-payment)
- [Payment Channels](#payment-channels)
- [Creating a Payment Intent](#creating-a-payment-intent)
- [Handling Callbacks](#handling-callbacks)
- [Payment & Transaction Status](#payment--transaction-status)
- [Transactions](#transactions)
- [FPX Direct Debit](#fpx-direct-debit)
- [Manual Bank Transfer](#manual-bank-transfer)
- [Portals & FPX Banks](#portals--fpx-banks)
- [Error Handling](#error-handling)
- [Response Objects](#response-objects)
- [Security Recommendations](#security-recommendations)

## Requirements

- Rust 1.75+ and an async runtime (examples use [`tokio`](https://tokio.rs)).
- Two credentials from your Bayarcash console:
  - **API token** — authenticates SDK requests.
  - **API secret key** — signs request checksums and verifies callbacks.

## Installation

```toml
# Cargo.toml
[dependencies]
bayarcash = "0.1"
tokio = { version = "1", features = ["full"] }
```

The crate uses `reqwest` with `rustls` (no system OpenSSL dependency).

## Getting Started

```rust
use bayarcash::{Bayarcash, ApiVersion};

let client = Bayarcash::new("YOUR_API_TOKEN")
    .use_sandbox()               // remove this in production
    .set_api_version(ApiVersion::V3) // 'V2' (default) or 'V3'
    .set_timeout(60);            // request timeout in seconds (default 30)

assert_eq!(client.api_version(), ApiVersion::V3);
```

> Configure the client (`use_sandbox()` / `set_api_version()`) before making requests. Omit `use_sandbox()` in production to hit the live gateway.

## Quick Start: Accept a Payment

A complete FPX payment flow, from creating the payment to redirecting the payer:

```rust
use bayarcash::{Bayarcash, PaymentIntentRequest};

# async fn run() -> Result<(), bayarcash::Error> {
let client = Bayarcash::new("YOUR_API_TOKEN").use_sandbox();
let api_secret_key = "YOUR_API_SECRET_KEY";

// 1. Build the payment request
// 2. Sign it (recommended) with `.signed(...)`
let request = PaymentIntentRequest::new(
        "your_portal_key",
        "INV-1001",           // order_number (max 30 chars)
        "10.00",              // amount
        "Ahmad bin Abdullah", // payer_name
        "ahmad@example.com",  // payer_email
    )
    .payment_channel(Bayarcash::FPX)
    .payer_telephone_number("0123456789")
    .return_url("https://your-site.com/payment/return")
    .callback_url("https://your-site.com/payment/callback")
    .signed(api_secret_key);

// 3. Create the payment intent and redirect the payer to Bayarcash
let intent = client.create_payment_intent(&request).await?;
if let Some(url) = intent.url {
    // redirect the payer's browser to `url`
    println!("Redirect to: {url}");
}
# Ok(())
# }
```

After payment, Bayarcash calls your `callback_url` (server-to-server) and redirects the payer to your `return_url`. Verify both — see [Handling Callbacks](#handling-callbacks).

## Payment Channels

Pass one of these constants (or a `Vec<i32>` of them) as the payment channel. They are available both as `Bayarcash::FPX` and in the [`channels`] module.

```rust
Bayarcash::FPX                 // 1  — FPX Online Banking
Bayarcash::MANUAL_TRANSFER     // 2  — Manual Bank Transfer
Bayarcash::FPX_DIRECT_DEBIT    // 3  — FPX Direct Debit
Bayarcash::FPX_LINE_OF_CREDIT  // 4  — FPX Line of Credit
Bayarcash::DUITNOW_DOBW        // 5  — DuitNow Online Banking
Bayarcash::DUITNOW_QR          // 6  — DuitNow QR
Bayarcash::SPAYLATER           // 7  — ShopeePayLater
Bayarcash::BOOST_PAYFLEX       // 8  — Boost PayFlex
Bayarcash::QRISOB              // 9  — QRIS Online Banking
Bayarcash::QRISWALLET          // 10 — QRIS Wallet
Bayarcash::NETS                // 11 — NETS
Bayarcash::CREDIT_CARD         // 12 — Credit Card
Bayarcash::ALIPAY              // 13 — Alipay
Bayarcash::WECHATPAY           // 14 — WeChat Pay
Bayarcash::PROMPTPAY           // 15 — PromptPay
Bayarcash::TOUCH_N_GO          // 16 — Touch 'n Go eWallet
Bayarcash::BOOST_WALLET        // 17 — Boost Wallet
Bayarcash::GRABPAY             // 18 — GrabPay
Bayarcash::GRABPL              // 19 — Grab PayLater
Bayarcash::SHOPEE_PAY          // 21 — ShopeePay (note: there is no channel 20)
```

Select multiple channels by passing a vector:

```rust
# use bayarcash::{Bayarcash, PaymentIntentRequest};
# let req = PaymentIntentRequest::new("p", "o", "10.00", "n", "e@x.com")
.payment_channel(vec![Bayarcash::FPX, Bayarcash::DUITNOW_DOBW])
# ;
```

## Creating a Payment Intent

`PaymentIntentRequest::new(portal_key, order_number, amount, payer_name, payer_email)` takes the required fields; the rest are builder methods:

| Field | Required | Description |
|---|---|---|
| `portal_key` | ✅ | Your portal key. |
| `order_number` | ✅ | Your reference. Max 30 chars. |
| `amount` | ✅ | String with up to 2 decimals, e.g. `"10.00"`. |
| `payer_name` | ✅ | Max 150 chars. |
| `payer_email` | ✅ | Valid email, max 250 chars. |
| `.payment_channel(...)` | ➖ | A channel id, or a `Vec<i32>` of ids. If omitted, the payer chooses. |
| `.payer_telephone_number(...)` | ➖ | Required for e-wallet / DuitNow channels. |
| `.return_url(...)` | ➖ | Browser redirect after payment. |
| `.callback_url(...)` | ➖ | Server-to-server notification URL. |
| `.metadata(...)` | ➖ | Extra data echoed back. |
| `.signed(secret)` / `.checksum(...)` | ➖ | Recommended. See below. |

### Checksum

The checksum protects the request from tampering. It is computed from `payment_channel`, `order_number`, `amount`, `payer_name`, and `payer_email` (HMAC-SHA256). Append it with `.signed(secret)`, or compute it explicitly:

```rust
# use bayarcash::{Bayarcash, PaymentIntentRequest};
# let client = Bayarcash::new("t");
# let mut request = PaymentIntentRequest::new("p", "o", "10.00", "n", "e@x.com");
let checksum = client.create_payment_intent_checksum_value("YOUR_API_SECRET_KEY", &request);
request = request.checksum(checksum);
```

## Handling Callbacks

Bayarcash sends notifications you must verify with your API secret key before trusting. Collect the form/query fields into any type implementing [`CallbackData`] — `HashMap<String, String>`, `serde_json::Value`, etc. — then call the matching verifier. Each returns `true` only when the checksum matches (constant-time comparison).

```rust
use bayarcash::Bayarcash;
use std::collections::HashMap;

# let client = Bayarcash::new("token");
# let callback_data: HashMap<String, String> = HashMap::new();
let secret = "YOUR_API_SECRET_KEY";

// Transaction callback (sent to your callback_url)
if client.verify_transaction_callback_data(&callback_data, secret) {
    // Data is authentic — safe to process.
}

// Payer redirect (sent to your return_url)
if client.verify_return_url_callback_data(&callback_data, secret) { /* ... */ }

// Pre-transaction callback
if client.verify_pre_transaction_callback_data(&callback_data, secret) { /* ... */ }
```

Mandate-specific verifiers: `verify_direct_debit_bank_approval_callback_data`, `verify_direct_debit_authorization_callback_data` (includes the `application_type` field), and `verify_direct_debit_transaction_callback_data`.

## Payment & Transaction Status

Status is an integer code. Use the helpers instead of hardcoding numbers:

```rust
use bayarcash::Fpx;

Fpx::STATUS_NEW;        // 0
Fpx::STATUS_PENDING;    // 1
Fpx::STATUS_FAILED;     // 2
Fpx::STATUS_SUCCESS;    // 3
Fpx::STATUS_CANCELLED;  // 4

assert_eq!(Fpx::status_text(Fpx::STATUS_SUCCESS), "Successful");
```

Transaction and mandate models expose `.status_code()` to parse the status string into an `i32`:

```rust
# use bayarcash::{Fpx, Transaction};
# let transaction = Transaction::default();
if transaction.status_code() == Some(Fpx::STATUS_SUCCESS) {
    // Payment successful
}
```

There are also [`FpxDirectDebit`] status/type helpers and [`duitnow::Dobw`] for DuitNow.

## Transactions

```rust
# use bayarcash::{Bayarcash, ApiVersion, TransactionFilters};
# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
// Single transaction (v2 and v3)
let transaction = client.get_transaction("transaction_id").await?;
# Ok(()) }
```

The following query helpers require **API v3** and return `Err(Error::Unsupported(_))` on v2:

```rust
# use bayarcash::{Bayarcash, ApiVersion, TransactionFilters};
# async fn run() -> Result<(), bayarcash::Error> {
let client = Bayarcash::new("YOUR_API_TOKEN").set_api_version(ApiVersion::V3);

let result = client.get_all_transactions(
    &TransactionFilters::new()
        .order_number("INV-1001")
        .status("3")
        .payment_channel(Bayarcash::FPX)
        .exchange_reference_number("REF123")
        .payer_email("ahmad@example.com"),
).await?;
// result.data => Vec<Transaction>, result.meta => pagination metadata

let by_order   = client.get_transaction_by_order_number("INV-1001").await?;
let by_email   = client.get_transactions_by_payer_email("ahmad@example.com").await?;
let by_status  = client.get_transactions_by_status("3").await?;
let by_channel = client.get_transactions_by_payment_channel(Bayarcash::FPX).await?;
let by_ref     = client.get_transaction_by_reference_number("REF123").await?; // Option<Transaction>

// Payment intents (v3 only)
let intent = client.get_payment_intent("payment_intent_id").await?;
let cancelled = client.cancel_payment_intent("payment_intent_id").await?;
# Ok(()) }
```

## FPX Direct Debit

Constants live on the [`FpxDirectDebit`] type:

```rust
use bayarcash::FpxDirectDebit;

// Payer ID type
FpxDirectDebit::NRIC;                  // 1
FpxDirectDebit::OLD_IC;                // 2
FpxDirectDebit::PASSPORT;              // 3
FpxDirectDebit::BUSINESS_REGISTRATION; // 4
FpxDirectDebit::OTHERS;                // 5

// Frequency mode
FpxDirectDebit::MODE_DAILY;   // "DL"
FpxDirectDebit::MODE_WEEKLY;  // "WK"
FpxDirectDebit::MODE_MONTHLY; // "MT"
FpxDirectDebit::MODE_YEARLY;  // "YR"
```

### 1. Enrolment

```rust
use bayarcash::{Bayarcash, FpxDirectDebit, FpxDirectDebitEnrolmentRequest};

# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
# let api_secret_key = "s";
let request = FpxDirectDebitEnrolmentRequest {
    portal_key: "your_portal_key".into(),
    order_number: "DD-1001".into(),
    amount: "10.00".into(), // range 5.00–30000.00
    payer_name: "Ahmad bin Abdullah".into(),
    payer_id_type: FpxDirectDebit::NRIC,
    payer_id: "900101011234".into(),
    payer_email: "ahmad@example.com".into(), // max 27 chars
    payer_telephone_number: "0123456789".into(),
    application_reason: "Monthly subscription".into(),
    frequency_mode: FpxDirectDebit::MODE_MONTHLY.into(),
    effective_date: Some("2026-08-01".into()),
    return_url: Some("https://your-site.com/mandate/return".into()),
    ..Default::default()
}
.signed(api_secret_key);

let mandate = client.create_fpx_direct_debit_enrollment(&request).await?;
// redirect the payer to `mandate.url`
# Ok(()) }
```

### 2. Maintenance

```rust
use bayarcash::{Bayarcash, FpxDirectDebit, FpxDirectDebitMaintenanceRequest};

# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
# let api_secret_key = "s";
let request = FpxDirectDebitMaintenanceRequest {
    amount: "15.00".into(),
    payer_email: "ahmad@example.com".into(),
    payer_telephone_number: "0123456789".into(),
    application_reason: "Update amount".into(),
    frequency_mode: FpxDirectDebit::MODE_MONTHLY.into(),
    ..Default::default()
}
.signed(api_secret_key);

let mandate = client.create_fpx_direct_debit_maintenance("mandate_id", &request).await?;
# Ok(()) }
```

### 3. Termination

```rust
use bayarcash::{Bayarcash, FpxDirectDebitTerminationRequest};

# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
let request = FpxDirectDebitTerminationRequest::new("Customer cancelled");
let mandate = client.create_fpx_direct_debit_termination("mandate_id", &request).await?;
# Ok(()) }
```

### Retrieving mandates

```rust
# use bayarcash::Bayarcash;
# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
let mandate     = client.get_fpx_direct_debit("mandate_id").await?;
let transaction = client.get_fpx_direct_debit_transaction("transaction_id").await?;
# Ok(()) }
```

## Manual Bank Transfer

Submit a manual (offline) bank transfer with proof of payment. `payment_gateway` must be `2` (`Bayarcash::MANUAL_TRANSFER`).

```rust
use bayarcash::{Bayarcash, ManualBankTransferRequest};

# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
let request = ManualBankTransferRequest {
    portal_key: "your_portal_key".into(),
    payment_gateway: Bayarcash::MANUAL_TRANSFER, // must be 2
    order_no: "MT-1001".into(),
    buyer_name: "Ahmad bin Abdullah".into(),
    buyer_email: "ahmad@example.com".into(),
    buyer_tel_no: Some("0123456789".into()),
    order_amount: "10.00".into(),
    merchant_bank_name: "Maybank".into(),
    merchant_bank_account: "1234567890".into(),
    merchant_bank_account_holder: "Your Company Sdn Bhd".into(),
    bank_transfer_type: "Internet Banking".into(),
    bank_transfer_notes: "Payment for order MT-1001".into(),
    bank_transfer_date: None, // defaults to today
    proof_of_payment: Some("/path/to/receipt.jpg".into()), // jpeg/png/gif/pdf
    ..Default::default()
};

let response = client.create_manual_bank_transfer(&request, false).await?;
# Ok(()) }
```

The response is a [`ManualBankTransferResponse`] enum (`HtmlForm`, `Json`, `Raw`, or `Redirect`). Update the status of an existing transfer:

```rust
use bayarcash::{Bayarcash, Fpx};

# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
client.update_manual_bank_transfer_status(
    "ref_no_here",
    &Fpx::STATUS_SUCCESS.to_string(),
    "10.00",
).await?;
# Ok(()) }
```

## Portals & FPX Banks

```rust
# use bayarcash::Bayarcash;
# async fn run() -> Result<(), bayarcash::Error> {
# let client = Bayarcash::new("t");
let portals  = client.get_portals().await?;              // all portals
let channels = client.get_channels("your_portal_key").await?; // channels for a portal
let banks    = client.fpx_banks_list().await?;           // FPX banks
# Ok(()) }
```

## Error Handling

Operations return `Result<T, bayarcash::Error>`. Match on the typed error:

```rust
use bayarcash::{Bayarcash, Error, PaymentIntentRequest};

# async fn run() {
# let client = Bayarcash::new("t");
# let request = PaymentIntentRequest::new("p", "o", "10.00", "n", "e@x.com");
match client.create_payment_intent(&request).await {
    Ok(intent) => { /* ... */ }
    Err(Error::Validation(v)) => {         // 422
        let _errors = v.errors();
    }
    Err(Error::NotFound) => {}             // 404
    Err(Error::RateLimitExceeded { resets_at }) => { // 429
        let _reset = resets_at;
    }
    Err(Error::FailedAction(message)) => { // 400
        eprintln!("{message}");
    }
    Err(other) => eprintln!("{other}"),
}
# }
```

| Variant | HTTP | Meaning |
|---|---|---|
| `Error::Validation` | 422 | Invalid data. Call `.errors()` for details. |
| `Error::FailedAction` | 400 | Request failed. Carries the gateway message. |
| `Error::NotFound` | 404 | Resource not found. |
| `Error::RateLimitExceeded` | 429 | Rate limited. `resets_at` holds the reset time. |
| `Error::Api` | other | Any other non-2xx response (status + body). |
| `Error::Unsupported` | — | A v3-only method was called on v2. |
| `Error::InvalidArgument` | — | A required argument was missing or invalid. |

## Response Objects

API methods return typed structs with `Option` fields; anything the gateway returns that is not modelled is preserved in each struct's `extra: HashMap<String, serde_json::Value>`. All models derive `serde::Serialize`, so `serde_json::to_value(&model)` is the equivalent of the PHP `toArray()`.

**`PaymentIntent`** — `url`, `id`, `status`, `amount`, `order_number`, `payer_name`, `payer_email`, …

**`Transaction`** — `id`, `status` (+ `.status_code()`), `status_description`, `amount`, `order_number`, `exchange_reference_number`, `payer_name`, `payer_email`, …

**`Mandate`**, **`MandateApplication`**, **`Portal`**, **`FpxBank`** mirror their PHP resource counterparts.

## Security Recommendations

1. Always send a `checksum` with payment and mandate requests (`.signed(secret)`).
2. Verify **every** callback with the provided verification methods before acting on it.
3. Store and check transaction ids to prevent duplicate processing.
4. Use HTTPS for your `return_url` and `callback_url`.
5. Keep your API token and secret key out of source control.

## API Documentation

For full API details, see the [Official Bayarcash API Documentation](https://api.webimpian.support/bayarcash).

## License

Open-sourced software licensed under the [MIT license](LICENSE).
