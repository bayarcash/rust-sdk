//! The [`Bayarcash`] client and its operations.

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::callback::{self, CallbackData};
use crate::channels;
use crate::checksum;
use crate::config::ApiVersion;
use crate::error::{Error, Result, ValidationErrors};
use crate::models::{
    FpxBank, Mandate, MandateApplication, PaymentIntent, Portal, Transaction, TransactionList,
};
use crate::requests::{
    FpxDirectDebitEnrolmentRequest, FpxDirectDebitMaintenanceRequest,
    FpxDirectDebitTerminationRequest, PaymentIntentRequest, TransactionFilters,
};

/// The Bayarcash API client.
///
/// Construct with [`Bayarcash::new`], then configure fluently:
///
/// ```
/// use bayarcash::{Bayarcash, ApiVersion};
///
/// let client = Bayarcash::new("YOUR_API_TOKEN")
///     .use_sandbox()          // remove in production
///     .set_api_version(ApiVersion::V3)
///     .set_timeout(60);
/// ```
#[derive(Debug, Clone)]
pub struct Bayarcash {
    pub(crate) token: String,
    pub(crate) sandbox: bool,
    api_version: ApiVersion,
    pub(crate) timeout: Duration,
    pub(crate) http: reqwest::Client,
    base_url_override: Option<String>,
}

impl Bayarcash {
    // -- Payment channel constants (mirror `Bayarcash::FPX` etc.) --------------

    /// FPX Online Banking.
    pub const FPX: i32 = channels::FPX;
    /// Manual Bank Transfer.
    pub const MANUAL_TRANSFER: i32 = channels::MANUAL_TRANSFER;
    /// FPX Direct Debit.
    pub const FPX_DIRECT_DEBIT: i32 = channels::FPX_DIRECT_DEBIT;
    /// FPX Line of Credit.
    pub const FPX_LINE_OF_CREDIT: i32 = channels::FPX_LINE_OF_CREDIT;
    /// DuitNow Online Banking / Wallets.
    pub const DUITNOW_DOBW: i32 = channels::DUITNOW_DOBW;
    /// DuitNow QR.
    pub const DUITNOW_QR: i32 = channels::DUITNOW_QR;
    /// ShopeePayLater.
    pub const SPAYLATER: i32 = channels::SPAYLATER;
    /// Boost PayFlex.
    pub const BOOST_PAYFLEX: i32 = channels::BOOST_PAYFLEX;
    /// QRIS Online Banking.
    pub const QRISOB: i32 = channels::QRISOB;
    /// QRIS Wallet.
    pub const QRISWALLET: i32 = channels::QRISWALLET;
    /// NETS.
    pub const NETS: i32 = channels::NETS;
    /// Credit Card.
    pub const CREDIT_CARD: i32 = channels::CREDIT_CARD;
    /// Alipay.
    pub const ALIPAY: i32 = channels::ALIPAY;
    /// WeChat Pay.
    pub const WECHATPAY: i32 = channels::WECHATPAY;
    /// PromptPay.
    pub const PROMPTPAY: i32 = channels::PROMPTPAY;
    /// Touch 'n Go eWallet.
    pub const TOUCH_N_GO: i32 = channels::TOUCH_N_GO;
    /// Boost Wallet.
    pub const BOOST_WALLET: i32 = channels::BOOST_WALLET;
    /// GrabPay.
    pub const GRABPAY: i32 = channels::GRABPAY;
    /// Grab PayLater.
    pub const GRABPL: i32 = channels::GRABPL;
    /// ShopeePay (note: there is no channel id 20).
    pub const SHOPEE_PAY: i32 = channels::SHOPEE_PAY;

    // -- Construction & configuration -----------------------------------------

    /// Create a new client with the given API token (production, v2, 30s timeout).
    pub fn new(token: impl Into<String>) -> Self {
        Bayarcash {
            token: token.into(),
            sandbox: false,
            api_version: ApiVersion::V2,
            timeout: Duration::from_secs(30),
            http: reqwest::Client::new(),
            base_url_override: None,
        }
    }

    /// Override the base URL (e.g. to route through a proxy or a mock server in
    /// tests). A trailing slash is added if missing. Pass this and requests go
    /// to `<base>/<endpoint>` instead of the computed gateway host.
    pub fn set_base_url(mut self, url: impl Into<String>) -> Self {
        let mut url = url.into();
        if !url.ends_with('/') {
            url.push('/');
        }
        self.base_url_override = Some(url);
        self
    }

    /// Replace the API token.
    pub fn set_token(mut self, token: impl Into<String>) -> Self {
        self.token = token.into();
        self
    }

    /// Switch to the sandbox environment.
    pub fn use_sandbox(mut self) -> Self {
        self.sandbox = true;
        self
    }

    /// Choose the API version (`v2` default, or `v3`).
    pub fn set_api_version(mut self, version: ApiVersion) -> Self {
        self.api_version = version;
        self
    }

    /// Set the per-request timeout, in seconds.
    pub fn set_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }

    /// The current API version.
    pub fn api_version(&self) -> ApiVersion {
        self.api_version
    }

    /// The current timeout, in seconds.
    pub fn timeout(&self) -> u64 {
        self.timeout.as_secs()
    }

    /// Whether the sandbox environment is in use.
    pub fn is_sandbox(&self) -> bool {
        self.sandbox
    }

    /// The base URI for the current version and environment.
    pub fn base_uri(&self) -> &'static str {
        match (self.api_version, self.sandbox) {
            (ApiVersion::V3, false) => "https://api.console.bayar.cash/v3/",
            (ApiVersion::V3, true) => "https://api.console.bayarcash-sandbox.com/v3/",
            (ApiVersion::V2, false) => "https://console.bayar.cash/api/v2/",
            (ApiVersion::V2, true) => "https://console.bayarcash-sandbox.com/api/v2/",
        }
    }

    fn require_v3(&self, method: &str) -> Result<()> {
        if self.api_version != ApiVersion::V3 {
            return Err(Error::Unsupported(format!(
                "The {method} method is only available for API version v3."
            )));
        }
        Ok(())
    }

    // -- Low-level HTTP --------------------------------------------------------

    async fn send(
        &self,
        method: Method,
        uri: &str,
        form: Option<&[(String, String)]>,
    ) -> Result<reqwest::Response> {
        let base = self.base_url_override.as_deref().unwrap_or(self.base_uri());
        let url = format!("{base}{uri}");
        let mut builder = self
            .http
            .request(method, &url)
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(&self.token)
            .timeout(self.timeout);

        if let Some(pairs) = form {
            builder = builder.form(pairs);
        }

        let response = builder.send().await?;
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(map_response_error(response).await)
        }
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        uri: &str,
        form: Option<&[(String, String)]>,
    ) -> Result<T> {
        let response = self.send(method, uri, form).await?;
        let bytes = response.bytes().await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn request_value(
        &self,
        method: Method,
        uri: &str,
        form: Option<&[(String, String)]>,
    ) -> Result<Value> {
        self.request_json(method, uri, form).await
    }

    // -- Portals & banks -------------------------------------------------------

    /// List all portals for your account (`get portals`).
    pub async fn get_portals(&self) -> Result<Vec<Portal>> {
        let response: Value = self.request_value(Method::GET, "portals", None).await?;
        let items = match response.get("data") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => match response {
                Value::Array(arr) => arr,
                other => vec![other],
            },
        };
        items
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(Error::from))
            .collect()
    }

    /// Payment channels available for a portal, matched by `portal_key`.
    ///
    /// Returns the raw channel objects (as the PHP SDK does), or an empty vec
    /// when the portal is not found.
    pub async fn get_channels(&self, portal_key: &str) -> Result<Vec<Value>> {
        let portals = self.get_portals().await?;
        for portal in portals {
            if portal.portal_key.as_deref() == Some(portal_key) {
                return Ok(portal.payment_channels.unwrap_or_default());
            }
        }
        Ok(Vec::new())
    }

    /// List FPX banks (`get banks`).
    pub async fn fpx_banks_list(&self) -> Result<Vec<FpxBank>> {
        self.request_json(Method::GET, "banks", None).await
    }

    // -- Payment intents -------------------------------------------------------

    /// Create a payment intent (`post payment-intents`).
    pub async fn create_payment_intent(
        &self,
        request: &PaymentIntentRequest,
    ) -> Result<PaymentIntent> {
        let form = request.to_form_pairs();
        self.request_json(Method::POST, "payment-intents", Some(&form))
            .await
    }

    /// Retrieve a payment intent by id (**v3 only**).
    pub async fn get_payment_intent(&self, payment_intent_id: &str) -> Result<PaymentIntent> {
        self.require_v3("getPaymentIntent")?;
        let uri = format!("payment-intents/{payment_intent_id}");
        self.request_json(Method::GET, &uri, None).await
    }

    /// Cancel a payment intent by id (**v3 only**).
    pub async fn cancel_payment_intent(&self, payment_intent_id: &str) -> Result<PaymentIntent> {
        self.require_v3("cancelPaymentIntent")?;
        let uri = format!("payment-intents/{payment_intent_id}");
        self.request_json(Method::DELETE, &uri, None).await
    }

    // -- Transactions ----------------------------------------------------------

    /// Retrieve a single transaction by id (v2 and v3).
    pub async fn get_transaction(&self, id: &str) -> Result<Transaction> {
        let uri = format!("transactions/{id}");
        self.request_json(Method::GET, &uri, None).await
    }

    /// Retrieve all transactions with optional filters (**v3 only**).
    pub async fn get_all_transactions(
        &self,
        filters: &TransactionFilters,
    ) -> Result<TransactionList> {
        self.require_v3("getAllTransactions")?;
        let query = build_query(&filters.query_pairs());
        let uri = if query.is_empty() {
            "transactions".to_string()
        } else {
            format!("transactions?{query}")
        };
        let response: Value = self.request_value(Method::GET, &uri, None).await?;
        let data = data_array(&response);
        let transactions = data
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(Error::from))
            .collect::<Result<Vec<Transaction>>>()?;
        let meta = response.get("meta").cloned().unwrap_or(Value::Null);
        Ok(TransactionList {
            data: transactions,
            meta,
        })
    }

    /// Retrieve transactions by order number (**v3 only**).
    pub async fn get_transaction_by_order_number(
        &self,
        order_number: &str,
    ) -> Result<Vec<Transaction>> {
        self.require_v3("getTransactionByOrderNumber")?;
        self.query_transactions("order_number", order_number).await
    }

    /// Retrieve transactions by payer email (**v3 only**).
    pub async fn get_transactions_by_payer_email(&self, email: &str) -> Result<Vec<Transaction>> {
        self.require_v3("getTransactionsByPayerEmail")?;
        self.query_transactions("payer_email", email).await
    }

    /// Retrieve transactions by status (**v3 only**).
    pub async fn get_transactions_by_status(&self, status: &str) -> Result<Vec<Transaction>> {
        self.require_v3("getTransactionsByStatus")?;
        self.query_transactions("status", status).await
    }

    /// Retrieve transactions by payment channel id (**v3 only**).
    pub async fn get_transactions_by_payment_channel(
        &self,
        channel: i32,
    ) -> Result<Vec<Transaction>> {
        self.require_v3("getTransactionsByPaymentChannel")?;
        self.query_transactions("payment_channel", &channel.to_string())
            .await
    }

    /// Retrieve a single transaction by exchange reference number (**v3 only**).
    ///
    /// Returns `None` when nothing matches.
    pub async fn get_transaction_by_reference_number(
        &self,
        reference_number: &str,
    ) -> Result<Option<Transaction>> {
        self.require_v3("getTransactionByReferenceNumber")?;
        let mut list = self
            .query_transactions("exchange_reference_number", reference_number)
            .await?;
        if list.is_empty() {
            Ok(None)
        } else {
            Ok(Some(list.remove(0)))
        }
    }

    async fn query_transactions(&self, key: &str, value: &str) -> Result<Vec<Transaction>> {
        let uri = format!("transactions?{}={}", key, urlencode(value));
        let response: Value = self.request_value(Method::GET, &uri, None).await?;
        data_array(&response)
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(Error::from))
            .collect()
    }

    // -- FPX Direct Debit ------------------------------------------------------

    /// Create an FPX Direct Debit enrolment (`post mandates`).
    pub async fn create_fpx_direct_debit_enrollment(
        &self,
        request: &FpxDirectDebitEnrolmentRequest,
    ) -> Result<MandateApplication> {
        let form = request.to_form_pairs();
        self.request_json(Method::POST, "mandates", Some(&form))
            .await
    }

    /// Update (maintain) an existing mandate (`put mandates/{id}`).
    pub async fn create_fpx_direct_debit_maintenance(
        &self,
        mandate_id: &str,
        request: &FpxDirectDebitMaintenanceRequest,
    ) -> Result<MandateApplication> {
        let form = request.to_form_pairs();
        let uri = format!("mandates/{mandate_id}");
        self.request_json(Method::PUT, &uri, Some(&form)).await
    }

    /// Terminate an existing mandate (`delete mandates/{id}`).
    pub async fn create_fpx_direct_debit_termination(
        &self,
        mandate_id: &str,
        request: &FpxDirectDebitTerminationRequest,
    ) -> Result<MandateApplication> {
        let form = request.to_form_pairs();
        let uri = format!("mandates/{mandate_id}");
        self.request_json(Method::DELETE, &uri, Some(&form)).await
    }

    /// Retrieve a mandate by id (`get mandates/{id}`).
    pub async fn get_fpx_direct_debit(&self, id: &str) -> Result<Mandate> {
        let uri = format!("mandates/{id}");
        self.request_json(Method::GET, &uri, None).await
    }

    /// Retrieve a mandate transaction by id (`get mandates/transactions/{id}`).
    pub async fn get_fpx_direct_debit_transaction(&self, id: &str) -> Result<Transaction> {
        let uri = format!("mandates/transactions/{id}");
        self.request_json(Method::GET, &uri, None).await
    }

    // -- Checksums (delegate to the `checksum` module) -------------------------

    /// Generic checksum: sort by key, join values with `|`, HMAC-SHA256.
    pub fn create_checksum_value(
        &self,
        secret_key: &str,
        payload: &BTreeMap<String, String>,
    ) -> String {
        checksum::create_checksum_value(secret_key, payload)
    }

    /// Payment-intent checksum.
    pub fn create_payment_intent_checksum_value(
        &self,
        secret_key: &str,
        request: &PaymentIntentRequest,
    ) -> String {
        checksum::create_payment_intent_checksum(secret_key, request)
    }

    /// FPX Direct Debit enrolment checksum.
    pub fn create_fpx_direct_debit_enrolment_checksum_value(
        &self,
        secret_key: &str,
        request: &FpxDirectDebitEnrolmentRequest,
    ) -> String {
        checksum::create_fpx_direct_debit_enrolment_checksum(secret_key, request)
    }

    /// FPX Direct Debit maintenance checksum.
    pub fn create_fpx_direct_debit_maintenance_checksum_value(
        &self,
        secret_key: &str,
        request: &FpxDirectDebitMaintenanceRequest,
    ) -> String {
        checksum::create_fpx_direct_debit_maintenance_checksum(secret_key, request)
    }

    // -- Callback verification (delegate to the `callback` module) -------------

    /// Verify a transaction callback (`callback_url`).
    pub fn verify_transaction_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_transaction_callback_data(data, secret_key)
    }

    /// Verify a return-URL callback (payer redirect).
    pub fn verify_return_url_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_return_url_callback_data(data, secret_key)
    }

    /// Verify a pre-transaction callback.
    pub fn verify_pre_transaction_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_pre_transaction_callback_data(data, secret_key)
    }

    /// Verify a direct-debit bank-approval callback.
    pub fn verify_direct_debit_bank_approval_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_direct_debit_bank_approval_callback_data(data, secret_key)
    }

    /// Verify a direct-debit authorization callback (includes `application_type`).
    pub fn verify_direct_debit_authorization_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_direct_debit_authorization_callback_data(data, secret_key)
    }

    /// Verify a direct-debit transaction callback.
    pub fn verify_direct_debit_transaction_callback_data(
        &self,
        data: &impl CallbackData,
        secret_key: &str,
    ) -> bool {
        callback::verify_direct_debit_transaction_callback_data(data, secret_key)
    }
}

// ---- shared helpers ---------------------------------------------------------

/// Map a non-2xx response to a typed [`Error`], mirroring `handleRequestError`.
pub(crate) async fn map_response_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    let reset = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<i64>().ok());
    let body = response.text().await.unwrap_or_default();

    match status {
        422 => {
            let value = serde_json::from_str(&body).unwrap_or(Value::Null);
            Error::Validation(ValidationErrors::from_value(value))
        }
        404 => Error::NotFound,
        400 => Error::FailedAction(extract_failed_action_message(&body)),
        429 => Error::RateLimitExceeded { resets_at: reset },
        _ => Error::Api { status, body },
    }
}

/// Extract the 400 error message the way the PHP SDK does:
/// prefer `message`, then `error`, else the raw body; array messages are JSON-encoded.
fn extract_failed_action_message(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(map)) => {
            let candidate = map.get("message").or_else(|| map.get("error"));
            match candidate {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => body.to_string(),
            }
        }
        _ => body.to_string(),
    }
}

/// `response["data"]` as an array, or an empty vec (matches `$response['data'] ?? []`).
fn data_array(response: &Value) -> Vec<Value> {
    response
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Build a URL query string from `(key, value)` pairs (values percent-encoded).
fn build_query(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-encode a string like PHP's `urlencode` (RFC 1738: space -> `+`).
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_uri_matrix() {
        let v2 = Bayarcash::new("t");
        assert_eq!(v2.base_uri(), "https://console.bayar.cash/api/v2/");
        assert_eq!(
            v2.clone().use_sandbox().base_uri(),
            "https://console.bayarcash-sandbox.com/api/v2/"
        );

        let v3 = Bayarcash::new("t").set_api_version(ApiVersion::V3);
        assert_eq!(v3.base_uri(), "https://api.console.bayar.cash/v3/");
        assert_eq!(
            v3.use_sandbox().base_uri(),
            "https://api.console.bayarcash-sandbox.com/v3/"
        );
    }

    #[test]
    fn channel_constants() {
        assert_eq!(Bayarcash::FPX, 1);
        assert_eq!(Bayarcash::GRABPL, 19);
        assert_eq!(Bayarcash::SHOPEE_PAY, 21);
    }

    #[test]
    fn v3_only_methods_rejected_on_v2() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = Bayarcash::new("t");
        let err = rt.block_on(client.get_payment_intent("pi_1")).unwrap_err();
        match err {
            Error::Unsupported(msg) => {
                assert!(msg.contains("getPaymentIntent"));
                assert!(msg.contains("v3"));
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn urlencode_matches_php() {
        assert_eq!(urlencode("ahmad@example.com"), "ahmad%40example.com");
        assert_eq!(urlencode("INV-1001"), "INV-1001");
        assert_eq!(urlencode("a b"), "a+b");
    }
}
