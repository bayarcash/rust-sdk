//! Response data-transfer objects (DTOs).
//!
//! These mirror the PHP `Resources\*` classes. Fields the API may omit are
//! `Option`, and any properties not modelled here are preserved in the
//! `extra` map (mirroring the PHP resources' dynamic properties). Numeric and
//! string fields tolerate the gateway returning either JSON numbers or strings.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

// ---- flexible deserializers -------------------------------------------------

fn de_opt_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

fn de_opt_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

fn de_opt_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    })
}

fn de_opt_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Bool(b)) => Some(b),
        Some(serde_json::Value::Number(n)) => n.as_i64().map(|i| i != 0),
        Some(serde_json::Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" | "" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

// ---- Payment intent ---------------------------------------------------------

/// A payment intent (from `create_payment_intent` / `get_payment_intent` /
/// `cancel_payment_intent`). Port of PHP `PaymentIntentResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Payer name.
    #[serde(default)]
    pub payer_name: Option<String>,
    /// Payer email.
    #[serde(default)]
    pub payer_email: Option<String>,
    /// Payer telephone number.
    #[serde(default)]
    pub payer_telephone_number: Option<String>,
    /// Your order number.
    #[serde(default)]
    pub order_number: Option<String>,
    /// Amount.
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub amount: Option<f64>,
    /// Checkout URL to redirect the payer to.
    #[serde(default)]
    pub url: Option<String>,
    /// Object type.
    #[serde(default, rename = "type")]
    pub type_: Option<String>,
    /// Payment-intent id.
    #[serde(default)]
    pub id: Option<String>,
    /// Status.
    #[serde(default, deserialize_with = "de_opt_string")]
    pub status: Option<String>,
    /// Last attempt.
    #[serde(default)]
    pub last_attempt: Option<serde_json::Value>,
    /// When the intent was paid.
    #[serde(default)]
    pub paid_at: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Payment attempts.
    #[serde(default)]
    pub attempts: Option<Vec<serde_json::Value>>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---- Transaction ------------------------------------------------------------

/// A transaction (from `get_transaction` and the v3 transaction queries).
/// Port of PHP `TransactionResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction id.
    #[serde(default)]
    pub id: Option<String>,
    /// Updated-at timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Created-at timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Transaction datetime.
    #[serde(default)]
    pub datetime: Option<String>,
    /// Payer name.
    #[serde(default)]
    pub payer_name: Option<String>,
    /// Payer email.
    #[serde(default)]
    pub payer_email: Option<String>,
    /// Payer telephone number.
    #[serde(default)]
    pub payer_telephone_number: Option<String>,
    /// Your order number.
    #[serde(default)]
    pub order_number: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Amount.
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub amount: Option<f64>,
    /// Exchange reference number.
    #[serde(default)]
    pub exchange_reference_number: Option<String>,
    /// Exchange transaction id.
    #[serde(default)]
    pub exchange_transaction_id: Option<String>,
    /// Payer bank name.
    #[serde(default)]
    pub payer_bank_name: Option<String>,
    /// Status code (as a string; see [`crate::Fpx`] constants).
    #[serde(default, deserialize_with = "de_opt_string")]
    pub status: Option<String>,
    /// Human-readable status description.
    #[serde(default)]
    pub status_description: Option<String>,
    /// Return URL.
    #[serde(default)]
    pub return_url: Option<String>,
    /// Metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Payout details.
    #[serde(default)]
    pub payout: Option<serde_json::Value>,
    /// Payment gateway details.
    #[serde(default)]
    pub payment_gateway: Option<serde_json::Value>,
    /// Portal identifier.
    #[serde(default)]
    pub portal: Option<String>,
    /// Merchant details.
    #[serde(default)]
    pub merchant: Option<serde_json::Value>,
    /// Mandate details (for direct-debit transactions).
    #[serde(default)]
    pub mandate: Option<serde_json::Value>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Transaction {
    /// Parse [`Transaction::status`] into an `i32`, if present and numeric.
    pub fn status_code(&self) -> Option<i32> {
        self.status.as_ref()?.trim().parse().ok()
    }
}

/// Paginated transaction list returned by `get_all_transactions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionList {
    /// The transactions on this page.
    #[serde(default)]
    pub data: Vec<Transaction>,
    /// Pagination metadata.
    #[serde(default)]
    pub meta: serde_json::Value,
}

// ---- Portal -----------------------------------------------------------------

/// A portal (from `get_portals`). Port of PHP `PortalResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Portal {
    /// Portal id.
    #[serde(default)]
    pub id: Option<String>,
    /// Created-at timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Portal key.
    #[serde(default)]
    pub portal_key: Option<String>,
    /// Portal name.
    #[serde(default)]
    pub portal_name: Option<String>,
    /// Website URL.
    #[serde(default)]
    pub website_url: Option<String>,
    /// Transaction notification email.
    #[serde(default)]
    pub transaction_notification_email: Option<String>,
    /// Secondary transaction notification email.
    #[serde(default)]
    pub secondary_transaction_notification_email: Option<String>,
    /// Custom payment button text.
    #[serde(default)]
    pub custom_payment_button_text: Option<String>,
    /// Whether SMS on successful transaction is enabled.
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub enabled_sms_on_successful_transaction: Option<i64>,
    /// Whether split payment is enabled.
    #[serde(default, deserialize_with = "de_opt_bool")]
    pub split_payment_enabled: Option<bool>,
    /// Split payment merchants.
    #[serde(default)]
    pub split_payment_merchants: Option<Vec<serde_json::Value>>,
    /// Payment channels available on this portal.
    #[serde(default)]
    pub payment_channels: Option<Vec<serde_json::Value>>,
    /// Merchant details.
    #[serde(default)]
    pub merchant: Option<serde_json::Value>,
    /// Portal URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Merchant id.
    #[serde(default)]
    pub merchant_id: Option<String>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---- FPX bank ---------------------------------------------------------------

/// An FPX bank (from `fpx_banks_list`). Port of PHP `FpxBankResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FpxBank {
    /// Bank name.
    #[serde(default)]
    pub bank_name: Option<String>,
    /// Bank display name.
    #[serde(default)]
    pub bank_display_name: Option<String>,
    /// Bank code.
    #[serde(default)]
    pub bank_code: Option<String>,
    /// Hashed bank code.
    #[serde(default)]
    pub bank_code_hashed: Option<String>,
    /// Whether the bank is currently available.
    #[serde(default, deserialize_with = "de_opt_bool")]
    pub bank_availability: Option<bool>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---- Mandate (FPX Direct Debit) ---------------------------------------------

/// A mandate (from `get_fpx_direct_debit`). Port of PHP `FpxDirectDebitResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mandate {
    /// Mandate id.
    #[serde(default)]
    pub id: Option<String>,
    /// Updated-at timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Mandate reference number.
    #[serde(default)]
    pub mandate_reference_number: Option<String>,
    /// Your order number.
    #[serde(default)]
    pub order_number: Option<String>,
    /// Application reason.
    #[serde(default)]
    pub application_reason: Option<String>,
    /// Frequency mode code.
    #[serde(default)]
    pub frequency_mode: Option<String>,
    /// Frequency mode label.
    #[serde(default)]
    pub frequency_mode_label: Option<String>,
    /// Effective date.
    #[serde(default)]
    pub effective_date: Option<String>,
    /// Expiry date.
    #[serde(default)]
    pub expiry_date: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Amount.
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub amount: Option<f64>,
    /// Payer name.
    #[serde(default)]
    pub payer_name: Option<String>,
    /// Payer ID value.
    #[serde(default)]
    pub payer_id: Option<String>,
    /// Payer ID type.
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub payer_id_type: Option<i64>,
    /// Payer bank account number.
    #[serde(default)]
    pub payer_bank_account_number: Option<String>,
    /// Payer email.
    #[serde(default)]
    pub payer_email: Option<String>,
    /// Payer telephone number.
    #[serde(default)]
    pub payer_telephone_number: Option<String>,
    /// Status code (as a string; see [`crate::FpxDirectDebit`] constants).
    #[serde(default, deserialize_with = "de_opt_string")]
    pub status: Option<String>,
    /// Human-readable status description.
    #[serde(default)]
    pub status_description: Option<String>,
    /// Return URL.
    #[serde(default)]
    pub return_url: Option<String>,
    /// Metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Portal identifier.
    #[serde(default)]
    pub portal: Option<String>,
    /// Merchant details.
    #[serde(default)]
    pub merchant: Option<serde_json::Value>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Mandate {
    /// Parse [`Mandate::status`] into an `i32`, if present and numeric.
    pub fn status_code(&self) -> Option<i32> {
        self.status.as_ref()?.trim().parse().ok()
    }
}

/// A mandate application result (from enrolment / maintenance / termination).
/// Port of PHP `FpxDirectDebitApplicationResource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MandateApplication {
    /// Payer name.
    #[serde(default)]
    pub payer_name: Option<String>,
    /// Payer ID type.
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub payer_id_type: Option<i64>,
    /// Payer ID value.
    #[serde(default)]
    pub payer_id: Option<String>,
    /// Payer email.
    #[serde(default)]
    pub payer_email: Option<String>,
    /// Payer telephone number.
    #[serde(default)]
    pub payer_telephone_number: Option<String>,
    /// Your order number.
    #[serde(default)]
    pub order_number: Option<String>,
    /// Amount.
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub amount: Option<f64>,
    /// Application type code.
    #[serde(default)]
    pub application_type: Option<String>,
    /// Application reason.
    #[serde(default)]
    pub application_reason: Option<String>,
    /// Frequency mode code.
    #[serde(default)]
    pub frequency_mode: Option<String>,
    /// Effective date.
    #[serde(default)]
    pub effective_date: Option<String>,
    /// Expiry date.
    #[serde(default)]
    pub expiry_date: Option<String>,
    /// URL to redirect the payer to.
    #[serde(default)]
    pub url: Option<String>,
    /// Any other fields returned by the gateway.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---- Manual bank transfer ---------------------------------------------------

/// The varied result shapes of a manual bank transfer creation, mirroring the
/// PHP `processManualTransferResponse` return values.
#[derive(Debug, Clone)]
pub enum ManualBankTransferResponse {
    /// A 2xx response containing an auto-submitting HTML form.
    HtmlForm {
        /// The raw HTML form.
        html_form: String,
        /// Extracted hidden fields plus `form_id`/`return_url`.
        form_data: HashMap<String, String>,
        /// The form's `action` URL, if found.
        return_url: Option<String>,
    },
    /// A 2xx JSON response.
    Json(serde_json::Value),
    /// A 2xx plain-text/other response.
    Raw(String),
    /// A 3xx response when redirects were not followed.
    Redirect {
        /// The redirect target (from the `Location` header, falling back to the body).
        redirect_url: String,
    },
}
