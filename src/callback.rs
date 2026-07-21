//! Callback verification.
//!
//! One verifier per callback type, each using the exact field list from the PHP
//! SDK's `CallbackVerifications`. For every verifier the payload is built from
//! the listed fields, sorted by key, joined with `|`, HMAC-SHA256'd with the
//! secret key, and compared against the callback's `checksum` using a
//! **constant-time** comparison (matching PHP's `hash_equals`).

use std::collections::{BTreeMap, HashMap};

use crate::checksum::hmac_sha256_hex;

/// Read individual fields out of a callback payload.
///
/// Implemented for the common map types and [`serde_json::Value`]. Implement it
/// for your web framework's request type to verify callbacks directly.
pub trait CallbackData {
    /// Return the string value of `key`, or `None` if absent or null.
    fn field(&self, key: &str) -> Option<String>;
}

impl CallbackData for HashMap<String, String> {
    fn field(&self, key: &str) -> Option<String> {
        self.get(key).cloned()
    }
}

impl CallbackData for BTreeMap<String, String> {
    fn field(&self, key: &str) -> Option<String> {
        self.get(key).cloned()
    }
}

impl CallbackData for HashMap<String, serde_json::Value> {
    fn field(&self, key: &str) -> Option<String> {
        self.get(key).and_then(json_value_to_string)
    }
}

impl CallbackData for serde_json::Value {
    fn field(&self, key: &str) -> Option<String> {
        self.get(key).and_then(json_value_to_string)
    }
}

fn json_value_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        other => Some(other.to_string()),
    }
}

/// Constant-time byte-slice equality (mirrors PHP `hash_equals`).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Build the sorted-and-joined payload, HMAC it, and constant-time compare
/// against the supplied `checksum` field.
fn verify(data: &impl CallbackData, secret_key: &str, fields: &[&str]) -> bool {
    let mut payload = BTreeMap::new();
    for field in fields {
        payload.insert(
            (*field).to_string(),
            data.field(field).unwrap_or_default(),
        );
    }
    let joined = payload.values().cloned().collect::<Vec<_>>().join("|");
    let computed = hmac_sha256_hex(secret_key, &joined);
    let provided = data.field("checksum").unwrap_or_default();
    constant_time_eq(computed.as_bytes(), provided.as_bytes())
}

/// Field list for the transaction callback (`callback_url`).
const TRANSACTION_FIELDS: &[&str] = &[
    "record_type",
    "transaction_id",
    "exchange_reference_number",
    "exchange_transaction_id",
    "order_number",
    "currency",
    "amount",
    "payer_name",
    "payer_email",
    "payer_bank_name",
    "status",
    "status_description",
    "datetime",
];

/// Field list for the return-URL callback (payer redirect).
const RETURN_URL_FIELDS: &[&str] = &[
    "transaction_id",
    "exchange_reference_number",
    "exchange_transaction_id",
    "order_number",
    "currency",
    "amount",
    "payer_bank_name",
    "status",
    "status_description",
];

/// Field list for the pre-transaction callback.
const PRE_TRANSACTION_FIELDS: &[&str] =
    &["record_type", "exchange_reference_number", "order_number"];

/// Field list for the direct-debit bank-approval callback.
const DD_BANK_APPROVAL_FIELDS: &[&str] = &[
    "record_type",
    "approval_date",
    "approval_status",
    "mandate_id",
    "mandate_reference_number",
    "order_number",
    "payer_bank_code_hashed",
    "payer_bank_code",
    "payer_bank_account_no",
    "application_type",
];

/// Field list for the direct-debit authorization callback.
///
/// NOTE: this list includes `application_type`, which the plain transaction
/// callback does not.
const DD_AUTHORIZATION_FIELDS: &[&str] = &[
    "record_type",
    "transaction_id",
    "mandate_id",
    "application_type",
    "exchange_reference_number",
    "exchange_transaction_id",
    "order_number",
    "currency",
    "amount",
    "payer_name",
    "payer_email",
    "payer_bank_name",
    "status",
    "status_description",
    "datetime",
];

/// Field list for the direct-debit transaction callback.
const DD_TRANSACTION_FIELDS: &[&str] = &[
    "record_type",
    "batch_number",
    "mandate_id",
    "mandate_reference_number",
    "transaction_id",
    "datetime",
    "reference_number",
    "amount",
    "status",
    "status_description",
    "cycle",
];

/// Verify a transaction callback (delivered to your `callback_url`).
pub fn verify_transaction_callback_data(data: &impl CallbackData, secret_key: &str) -> bool {
    verify(data, secret_key, TRANSACTION_FIELDS)
}

/// Verify a return-URL callback (payer browser redirect).
pub fn verify_return_url_callback_data(data: &impl CallbackData, secret_key: &str) -> bool {
    verify(data, secret_key, RETURN_URL_FIELDS)
}

/// Verify a pre-transaction callback.
pub fn verify_pre_transaction_callback_data(data: &impl CallbackData, secret_key: &str) -> bool {
    verify(data, secret_key, PRE_TRANSACTION_FIELDS)
}

/// Verify a direct-debit bank-approval callback.
pub fn verify_direct_debit_bank_approval_callback_data(
    data: &impl CallbackData,
    secret_key: &str,
) -> bool {
    verify(data, secret_key, DD_BANK_APPROVAL_FIELDS)
}

/// Verify a direct-debit authorization callback (includes `application_type`).
pub fn verify_direct_debit_authorization_callback_data(
    data: &impl CallbackData,
    secret_key: &str,
) -> bool {
    verify(data, secret_key, DD_AUTHORIZATION_FIELDS)
}

/// Verify a direct-debit transaction callback.
pub fn verify_direct_debit_transaction_callback_data(
    data: &impl CallbackData,
    secret_key: &str,
) -> bool {
    verify(data, secret_key, DD_TRANSACTION_FIELDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test_secret_key_123";

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn transaction_round_trip_matches_php_vector() {
        let mut data = map(&[
            ("record_type", "transaction"),
            ("transaction_id", "trx_123"),
            ("exchange_reference_number", "REF123"),
            ("exchange_transaction_id", "ETX1"),
            ("order_number", "INV-1001"),
            ("currency", "MYR"),
            ("amount", "10.00"),
            ("payer_name", "Ahmad"),
            ("payer_email", "ahmad@example.com"),
            ("payer_bank_name", "Maybank"),
            ("status", "3"),
            ("status_description", "Successful"),
            ("datetime", "2026-07-22 10:00:00"),
        ]);
        data.insert(
            "checksum".to_string(),
            "c3101648c9c6ec0ce5c46e5a6201e214da3bf2d1f436a8e7e53f064b0abb4cd2".to_string(),
        );
        assert!(verify_transaction_callback_data(&data, SECRET));

        // Tamper with the amount -> verification must fail.
        let mut tampered = data.clone();
        tampered.insert("amount".to_string(), "9999.00".to_string());
        assert!(!verify_transaction_callback_data(&tampered, SECRET));
    }

    #[test]
    fn direct_debit_authorization_includes_application_type() {
        let mut data = map(&[
            ("record_type", "authorization"),
            ("transaction_id", "trx_999"),
            ("mandate_id", "mand_1"),
            ("application_type", "01"),
            ("exchange_reference_number", "REF999"),
            ("exchange_transaction_id", "ETX9"),
            ("order_number", "DD-1001"),
            ("currency", "MYR"),
            ("amount", "10.00"),
            ("payer_name", "Ahmad"),
            ("payer_email", "ahmad@example.com"),
            ("payer_bank_name", "Maybank"),
            ("status", "3"),
            ("status_description", "Successful"),
            ("datetime", "2026-07-22 10:00:00"),
        ]);
        data.insert(
            "checksum".to_string(),
            "a3ffdd3f2dc291eb780c13ce0085470d3cc638471cafea767e586c55e1931f27".to_string(),
        );
        assert!(verify_direct_debit_authorization_callback_data(&data, SECRET));

        // Removing application_type changes the payload -> must fail.
        let mut without = data.clone();
        without.remove("application_type");
        assert!(!verify_direct_debit_authorization_callback_data(&without, SECRET));
    }

    #[test]
    fn return_url_and_pre_transaction_and_dd_vectors() {
        let mut ret = map(&[
            ("transaction_id", "trx_123"),
            ("exchange_reference_number", "REF123"),
            ("exchange_transaction_id", "ETX1"),
            ("order_number", "INV-1001"),
            ("currency", "MYR"),
            ("amount", "10.00"),
            ("payer_bank_name", "Maybank"),
            ("status", "3"),
            ("status_description", "Successful"),
        ]);
        ret.insert(
            "checksum".to_string(),
            "a9768123e18fee86e8b765f4cee6c95fb443971d8a16ceeae844640be3c15d60".to_string(),
        );
        assert!(verify_return_url_callback_data(&ret, SECRET));

        let mut pre = map(&[
            ("record_type", "pre_transaction"),
            ("exchange_reference_number", "REF1"),
            ("order_number", "INV-1001"),
        ]);
        pre.insert(
            "checksum".to_string(),
            "df8c3ff8c660311dce07d4fc9a1396db2d354fc02e1fd21d4787ddf4182d2833".to_string(),
        );
        assert!(verify_pre_transaction_callback_data(&pre, SECRET));

        let mut bank = map(&[
            ("record_type", "bank_approval"),
            ("approval_date", "2026-07-22"),
            ("approval_status", "approved"),
            ("mandate_id", "mand_1"),
            ("mandate_reference_number", "MREF1"),
            ("order_number", "DD-1001"),
            ("payer_bank_code_hashed", "HASH"),
            ("payer_bank_code", "MB2U0227"),
            ("payer_bank_account_no", "1234567890"),
            ("application_type", "01"),
        ]);
        bank.insert(
            "checksum".to_string(),
            "c2b4acddd031bdb56e2b8c0f49a96670da1ae26a2dfa8af311e048f19c21cf75".to_string(),
        );
        assert!(verify_direct_debit_bank_approval_callback_data(&bank, SECRET));

        let mut ddtx = map(&[
            ("record_type", "dd_transaction"),
            ("batch_number", "B1"),
            ("mandate_id", "mand_1"),
            ("mandate_reference_number", "MREF1"),
            ("transaction_id", "trx_1"),
            ("datetime", "2026-07-22 10:00:00"),
            ("reference_number", "RN1"),
            ("amount", "10.00"),
            ("status", "3"),
            ("status_description", "Successful"),
            ("cycle", "1"),
        ]);
        ddtx.insert(
            "checksum".to_string(),
            "1f5465c6289e85eeeccf23488c22fa166d9f8c7bd4f00fcf32fc3692d364703f".to_string(),
        );
        assert!(verify_direct_debit_transaction_callback_data(&ddtx, SECRET));
    }

    #[test]
    fn works_with_json_value() {
        let data = serde_json::json!({
            "record_type": "pre_transaction",
            "exchange_reference_number": "REF1",
            "order_number": "INV-1001",
            "checksum": "df8c3ff8c660311dce07d4fc9a1396db2d354fc02e1fd21d4787ddf4182d2833"
        });
        assert!(verify_pre_transaction_callback_data(&data, SECRET));
    }
}
