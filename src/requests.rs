//! Request builders for the write operations.
//!
//! Each builder produces the exact `application/x-www-form-urlencoded` field
//! set the PHP SDK sends, and knows how to compute its own checksum payload so
//! the generated checksum is byte-compatible with the gateway.

use std::path::PathBuf;

use crate::checksum;

/// A payment channel selection: either a single channel id or several.
///
/// Mirrors the PHP behaviour where `payment_channel` may be a single int or an
/// array of ints. For the checksum the ids are comma-joined; for the request
/// body a single id is sent as `payment_channel=<id>` and multiple ids as
/// `payment_channel[0]=<id>&payment_channel[1]=<id>...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentChannel {
    /// A single channel id.
    Single(i32),
    /// Multiple channel ids.
    Multiple(Vec<i32>),
}

impl PaymentChannel {
    /// The comma-joined value used inside the checksum payload.
    pub fn checksum_value(&self) -> String {
        match self {
            PaymentChannel::Single(id) => id.to_string(),
            PaymentChannel::Multiple(ids) => ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    fn form_pairs(&self) -> Vec<(String, String)> {
        match self {
            PaymentChannel::Single(id) => vec![("payment_channel".to_string(), id.to_string())],
            PaymentChannel::Multiple(ids) => ids
                .iter()
                .enumerate()
                .map(|(i, id)| (format!("payment_channel[{i}]"), id.to_string()))
                .collect(),
        }
    }
}

impl From<i32> for PaymentChannel {
    fn from(id: i32) -> Self {
        PaymentChannel::Single(id)
    }
}

impl From<Vec<i32>> for PaymentChannel {
    fn from(ids: Vec<i32>) -> Self {
        PaymentChannel::Multiple(ids)
    }
}

impl From<&[i32]> for PaymentChannel {
    fn from(ids: &[i32]) -> Self {
        PaymentChannel::Multiple(ids.to_vec())
    }
}

fn push_opt(pairs: &mut Vec<(String, String)>, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        pairs.push((key.to_string(), v.clone()));
    }
}

/// Builder for [`crate::Bayarcash::create_payment_intent`].
///
/// The five required fields map onto the checksum payload
/// (`payment_channel`, `order_number`, `amount`, `payer_name`, `payer_email`).
#[derive(Debug, Clone, Default)]
pub struct PaymentIntentRequest {
    /// Your portal key.
    pub portal_key: String,
    /// One or more payment channels. `None` lets the payer choose on Bayarcash.
    pub payment_channel: Option<PaymentChannel>,
    /// Your order/reference number (max 30 chars).
    pub order_number: String,
    /// Amount as a string with up to 2 decimals, e.g. `"10.00"`.
    pub amount: String,
    /// Payer name (max 150 chars).
    pub payer_name: String,
    /// Payer email (max 250 chars).
    pub payer_email: String,
    /// Payer telephone number (required for some e-wallet/DuitNow channels).
    pub payer_telephone_number: Option<String>,
    /// Browser redirect URL after payment.
    pub return_url: Option<String>,
    /// Server-to-server callback URL.
    pub callback_url: Option<String>,
    /// Arbitrary metadata echoed back by the gateway.
    pub metadata: Option<String>,
    /// Any additional form fields to send verbatim.
    pub extra: Vec<(String, String)>,
    /// The request checksum. Set it via [`PaymentIntentRequest::signed`] or
    /// [`crate::Bayarcash::create_payment_intent_checksum_value`].
    pub checksum: Option<String>,
}

impl PaymentIntentRequest {
    /// Construct a request from the five required fields.
    pub fn new(
        portal_key: impl Into<String>,
        order_number: impl Into<String>,
        amount: impl Into<String>,
        payer_name: impl Into<String>,
        payer_email: impl Into<String>,
    ) -> Self {
        PaymentIntentRequest {
            portal_key: portal_key.into(),
            order_number: order_number.into(),
            amount: amount.into(),
            payer_name: payer_name.into(),
            payer_email: payer_email.into(),
            ..Default::default()
        }
    }

    /// Set the payment channel(s). Accepts an `i32`, `Vec<i32>` or [`PaymentChannel`].
    pub fn payment_channel(mut self, channel: impl Into<PaymentChannel>) -> Self {
        self.payment_channel = Some(channel.into());
        self
    }

    /// Set the payer telephone number.
    pub fn payer_telephone_number(mut self, value: impl Into<String>) -> Self {
        self.payer_telephone_number = Some(value.into());
        self
    }

    /// Set the return URL.
    pub fn return_url(mut self, value: impl Into<String>) -> Self {
        self.return_url = Some(value.into());
        self
    }

    /// Set the callback URL.
    pub fn callback_url(mut self, value: impl Into<String>) -> Self {
        self.callback_url = Some(value.into());
        self
    }

    /// Set the metadata value.
    pub fn metadata(mut self, value: impl Into<String>) -> Self {
        self.metadata = Some(value.into());
        self
    }

    /// Add an arbitrary extra form field.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }

    /// Set the checksum explicitly.
    pub fn checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Compute and attach the checksum using the given secret key.
    pub fn signed(mut self, secret_key: &str) -> Self {
        self.checksum = Some(checksum::create_payment_intent_checksum(secret_key, &self));
        self
    }

    /// The ordered `(key, value)` pairs sent as the form body.
    pub fn to_form_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("portal_key".to_string(), self.portal_key.clone()));
        if let Some(channel) = &self.payment_channel {
            pairs.extend(channel.form_pairs());
        }
        pairs.push(("order_number".to_string(), self.order_number.clone()));
        pairs.push(("amount".to_string(), self.amount.clone()));
        pairs.push(("payer_name".to_string(), self.payer_name.clone()));
        pairs.push(("payer_email".to_string(), self.payer_email.clone()));
        push_opt(&mut pairs, "payer_telephone_number", &self.payer_telephone_number);
        push_opt(&mut pairs, "return_url", &self.return_url);
        push_opt(&mut pairs, "callback_url", &self.callback_url);
        push_opt(&mut pairs, "metadata", &self.metadata);
        pairs.extend(self.extra.iter().cloned());
        push_opt(&mut pairs, "checksum", &self.checksum);
        pairs
    }
}

/// Builder for [`crate::Bayarcash::create_fpx_direct_debit_enrollment`].
#[derive(Debug, Clone, Default)]
pub struct FpxDirectDebitEnrolmentRequest {
    /// Your portal key.
    pub portal_key: String,
    /// Your order/reference number.
    pub order_number: String,
    /// Amount as a string, e.g. `"10.00"` (range 5.00–30000.00).
    pub amount: String,
    /// Payer name.
    pub payer_name: String,
    /// Payer ID type (see [`crate::FpxDirectDebit`] constants).
    pub payer_id_type: i32,
    /// Payer ID value (e.g. NRIC number).
    pub payer_id: String,
    /// Payer email (max 27 chars).
    pub payer_email: String,
    /// Payer telephone number.
    pub payer_telephone_number: String,
    /// Reason for the mandate application.
    pub application_reason: String,
    /// Frequency mode (see [`crate::FpxDirectDebit`] `MODE_*`).
    pub frequency_mode: String,
    /// Optional effective date (`YYYY-MM-DD`).
    pub effective_date: Option<String>,
    /// Optional expiry date (`YYYY-MM-DD`).
    pub expiry_date: Option<String>,
    /// Browser redirect URL after enrolment.
    pub return_url: Option<String>,
    /// Any additional form fields.
    pub extra: Vec<(String, String)>,
    /// The request checksum.
    pub checksum: Option<String>,
}

impl FpxDirectDebitEnrolmentRequest {
    /// Add an arbitrary extra form field.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }

    /// Set the checksum explicitly.
    pub fn checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Compute and attach the enrolment checksum using the given secret key.
    pub fn signed(mut self, secret_key: &str) -> Self {
        self.checksum = Some(checksum::create_fpx_direct_debit_enrolment_checksum(
            secret_key, &self,
        ));
        self
    }

    /// The ordered `(key, value)` pairs sent as the form body.
    pub fn to_form_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![
            ("portal_key".to_string(), self.portal_key.clone()),
            ("order_number".to_string(), self.order_number.clone()),
            ("amount".to_string(), self.amount.clone()),
            ("payer_name".to_string(), self.payer_name.clone()),
            ("payer_id_type".to_string(), self.payer_id_type.to_string()),
            ("payer_id".to_string(), self.payer_id.clone()),
            ("payer_email".to_string(), self.payer_email.clone()),
            (
                "payer_telephone_number".to_string(),
                self.payer_telephone_number.clone(),
            ),
            ("application_reason".to_string(), self.application_reason.clone()),
            ("frequency_mode".to_string(), self.frequency_mode.clone()),
        ];
        push_opt(&mut pairs, "effective_date", &self.effective_date);
        push_opt(&mut pairs, "expiry_date", &self.expiry_date);
        push_opt(&mut pairs, "return_url", &self.return_url);
        pairs.extend(self.extra.iter().cloned());
        push_opt(&mut pairs, "checksum", &self.checksum);
        pairs
    }
}

/// Builder for [`crate::Bayarcash::create_fpx_direct_debit_maintenance`].
#[derive(Debug, Clone, Default)]
pub struct FpxDirectDebitMaintenanceRequest {
    /// New amount.
    pub amount: String,
    /// Payer email.
    pub payer_email: String,
    /// Payer telephone number.
    pub payer_telephone_number: String,
    /// Reason for the maintenance.
    pub application_reason: String,
    /// Frequency mode (see [`crate::FpxDirectDebit`] `MODE_*`).
    pub frequency_mode: String,
    /// Any additional form fields.
    pub extra: Vec<(String, String)>,
    /// The request checksum.
    pub checksum: Option<String>,
}

impl FpxDirectDebitMaintenanceRequest {
    /// Add an arbitrary extra form field.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }

    /// Set the checksum explicitly.
    pub fn checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Compute and attach the maintenance checksum using the given secret key.
    pub fn signed(mut self, secret_key: &str) -> Self {
        self.checksum = Some(checksum::create_fpx_direct_debit_maintenance_checksum(
            secret_key, &self,
        ));
        self
    }

    /// The ordered `(key, value)` pairs sent as the form body.
    pub fn to_form_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![
            ("amount".to_string(), self.amount.clone()),
            ("payer_email".to_string(), self.payer_email.clone()),
            (
                "payer_telephone_number".to_string(),
                self.payer_telephone_number.clone(),
            ),
            ("application_reason".to_string(), self.application_reason.clone()),
            ("frequency_mode".to_string(), self.frequency_mode.clone()),
        ];
        pairs.extend(self.extra.iter().cloned());
        push_opt(&mut pairs, "checksum", &self.checksum);
        pairs
    }
}

/// Builder for [`crate::Bayarcash::create_fpx_direct_debit_termination`].
#[derive(Debug, Clone, Default)]
pub struct FpxDirectDebitTerminationRequest {
    /// Reason for the termination.
    pub application_reason: String,
    /// Any additional form fields.
    pub extra: Vec<(String, String)>,
}

impl FpxDirectDebitTerminationRequest {
    /// Construct a termination request from a reason.
    pub fn new(application_reason: impl Into<String>) -> Self {
        FpxDirectDebitTerminationRequest {
            application_reason: application_reason.into(),
            extra: Vec::new(),
        }
    }

    /// Add an arbitrary extra form field.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }

    /// The ordered `(key, value)` pairs sent as the form body.
    pub fn to_form_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![("application_reason".to_string(), self.application_reason.clone())];
        pairs.extend(self.extra.iter().cloned());
        pairs
    }
}

/// Filters for [`crate::Bayarcash::get_all_transactions`] (v3 only).
///
/// Only these five parameters are forwarded to the gateway, matching the PHP
/// `getAllTransactions` allow-list.
#[derive(Debug, Clone, Default)]
pub struct TransactionFilters {
    /// Filter by order number.
    pub order_number: Option<String>,
    /// Filter by status code (as a string, e.g. `"3"`).
    pub status: Option<String>,
    /// Filter by payment channel id.
    pub payment_channel: Option<i32>,
    /// Filter by exchange reference number.
    pub exchange_reference_number: Option<String>,
    /// Filter by payer email.
    pub payer_email: Option<String>,
}

impl TransactionFilters {
    /// A new, empty filter set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by order number.
    pub fn order_number(mut self, value: impl Into<String>) -> Self {
        self.order_number = Some(value.into());
        self
    }

    /// Filter by status.
    pub fn status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
        self
    }

    /// Filter by payment channel id.
    pub fn payment_channel(mut self, value: i32) -> Self {
        self.payment_channel = Some(value);
        self
    }

    /// Filter by exchange reference number.
    pub fn exchange_reference_number(mut self, value: impl Into<String>) -> Self {
        self.exchange_reference_number = Some(value.into());
        self
    }

    /// Filter by payer email.
    pub fn payer_email(mut self, value: impl Into<String>) -> Self {
        self.payer_email = Some(value.into());
        self
    }

    /// The allow-listed query pairs, in a stable order.
    pub(crate) fn query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(v) = &self.order_number {
            pairs.push(("order_number".to_string(), v.clone()));
        }
        if let Some(v) = &self.status {
            pairs.push(("status".to_string(), v.clone()));
        }
        if let Some(v) = &self.payment_channel {
            pairs.push(("payment_channel".to_string(), v.to_string()));
        }
        if let Some(v) = &self.exchange_reference_number {
            pairs.push(("exchange_reference_number".to_string(), v.clone()));
        }
        if let Some(v) = &self.payer_email {
            pairs.push(("payer_email".to_string(), v.clone()));
        }
        pairs
    }
}

/// Builder for [`crate::Bayarcash::create_manual_bank_transfer`].
///
/// `payment_gateway` must be `2` ([`crate::channels::MANUAL_TRANSFER`]).
#[derive(Debug, Clone, Default)]
pub struct ManualBankTransferRequest {
    /// Your portal key.
    pub portal_key: String,
    /// Must be `2` (Manual Bank Transfer).
    pub payment_gateway: i32,
    /// Your order number.
    pub order_no: String,
    /// Buyer name.
    pub buyer_name: String,
    /// Buyer email.
    pub buyer_email: String,
    /// Buyer telephone number (optional).
    pub buyer_tel_no: Option<String>,
    /// Order amount, e.g. `"10.00"`.
    pub order_amount: String,
    /// Destination bank name.
    pub merchant_bank_name: String,
    /// Destination bank account number.
    pub merchant_bank_account: String,
    /// Destination bank account holder name.
    pub merchant_bank_account_holder: String,
    /// Transfer type, e.g. `"Internet Banking"` or `"Cash Deposit Machine (CDM)"`.
    pub bank_transfer_type: String,
    /// Free-form notes.
    pub bank_transfer_notes: String,
    /// Transfer date (`YYYY-MM-DD`). Defaults to today when `None`.
    pub bank_transfer_date: Option<String>,
    /// Path to the proof-of-payment file (jpeg/png/gif/pdf, max 10 MB).
    pub proof_of_payment: Option<PathBuf>,
    /// Any additional text form fields.
    pub extra: Vec<(String, String)>,
}

impl ManualBankTransferRequest {
    /// Add an arbitrary extra text field.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }

    /// The non-file text fields, in insertion order, with defaults applied.
    pub(crate) fn text_fields(&self) -> Vec<(String, String)> {
        let mut pairs = vec![
            ("portal_key".to_string(), self.portal_key.clone()),
            ("payment_gateway".to_string(), self.payment_gateway.to_string()),
            ("order_no".to_string(), self.order_no.clone()),
            ("buyer_name".to_string(), self.buyer_name.clone()),
            ("buyer_email".to_string(), self.buyer_email.clone()),
        ];
        if let Some(tel) = &self.buyer_tel_no {
            pairs.push(("buyer_tel_no".to_string(), tel.clone()));
        }
        pairs.push(("order_amount".to_string(), self.order_amount.clone()));
        pairs.push(("merchant_bank_name".to_string(), self.merchant_bank_name.clone()));
        pairs.push((
            "merchant_bank_account".to_string(),
            self.merchant_bank_account.clone(),
        ));
        pairs.push((
            "merchant_bank_account_holder".to_string(),
            self.merchant_bank_account_holder.clone(),
        ));
        pairs.push(("bank_transfer_type".to_string(), self.bank_transfer_type.clone()));
        pairs.push(("bank_transfer_notes".to_string(), self.bank_transfer_notes.clone()));
        pairs.extend(self.extra.iter().cloned());
        pairs
    }
}
