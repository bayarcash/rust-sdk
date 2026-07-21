//! HMAC-SHA256 checksum generation.
//!
//! Byte-compatible with the PHP SDK's `ChecksumGenerator`:
//!
//! 1. build the payload map,
//! 2. sort it by **key** ascending,
//! 3. join the **values** with `|`,
//! 4. HMAC-SHA256 with the secret key,
//! 5. lowercase hex output.
//!
//! A [`std::collections::BTreeMap`] iterates in ascending key order, which
//! matches PHP's `ksort()` for the ASCII field names used here.

use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::requests::{
    FpxDirectDebitEnrolmentRequest, FpxDirectDebitMaintenanceRequest, PaymentIntentRequest,
};

type HmacSha256 = Hmac<Sha256>;

/// Compute `hex(HMAC-SHA256(message, secret))`.
pub fn hmac_sha256_hex(secret_key: &str, message: &str) -> String {
    // `new_from_slice` only errors for algorithms with a fixed key size; HMAC
    // accepts any key length, so this never fails.
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Generic checksum: sort the map by key, join values with `|`, HMAC-SHA256.
///
/// Port of `createChecksumValue`.
pub fn create_checksum_value(secret_key: &str, payload: &BTreeMap<String, String>) -> String {
    let joined = payload
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("|");
    hmac_sha256_hex(secret_key, &joined)
}

/// Payment-intent checksum.
///
/// Signs `payment_channel` (comma-joined, empty when absent), `order_number`,
/// `amount`, `payer_name`, `payer_email`. Port of `createPaymentIntentChecksumValue`.
pub fn create_payment_intent_checksum(secret_key: &str, req: &PaymentIntentRequest) -> String {
    let mut payload = BTreeMap::new();
    payload.insert(
        "payment_channel".to_string(),
        req.payment_channel
            .as_ref()
            .map(|c| c.checksum_value())
            .unwrap_or_default(),
    );
    payload.insert("order_number".to_string(), req.order_number.clone());
    payload.insert("amount".to_string(), req.amount.clone());
    payload.insert("payer_name".to_string(), req.payer_name.clone());
    payload.insert("payer_email".to_string(), req.payer_email.clone());
    create_checksum_value(secret_key, &payload)
}

/// FPX Direct Debit enrolment checksum.
///
/// Port of `createFpxDirectDebitEnrolmentChecksumValue`.
pub fn create_fpx_direct_debit_enrolment_checksum(
    secret_key: &str,
    req: &FpxDirectDebitEnrolmentRequest,
) -> String {
    let mut payload = BTreeMap::new();
    payload.insert("order_number".to_string(), req.order_number.clone());
    payload.insert("amount".to_string(), req.amount.clone());
    payload.insert("payer_name".to_string(), req.payer_name.clone());
    payload.insert("payer_email".to_string(), req.payer_email.clone());
    payload.insert(
        "payer_telephone_number".to_string(),
        req.payer_telephone_number.clone(),
    );
    payload.insert("payer_id_type".to_string(), req.payer_id_type.to_string());
    payload.insert("payer_id".to_string(), req.payer_id.clone());
    payload.insert("application_reason".to_string(), req.application_reason.clone());
    payload.insert("frequency_mode".to_string(), req.frequency_mode.clone());
    create_checksum_value(secret_key, &payload)
}

/// FPX Direct Debit maintenance checksum.
///
/// Port of `createFpxDirectDebitMaintenanceChecksumValue`.
pub fn create_fpx_direct_debit_maintenance_checksum(
    secret_key: &str,
    req: &FpxDirectDebitMaintenanceRequest,
) -> String {
    let mut payload = BTreeMap::new();
    payload.insert("amount".to_string(), req.amount.clone());
    payload.insert("payer_email".to_string(), req.payer_email.clone());
    payload.insert(
        "payer_telephone_number".to_string(),
        req.payer_telephone_number.clone(),
    );
    payload.insert("application_reason".to_string(), req.application_reason.clone());
    payload.insert("frequency_mode".to_string(), req.frequency_mode.clone());
    create_checksum_value(secret_key, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test_secret_key_123";

    #[test]
    fn payment_intent_single_channel_matches_php_vector() {
        let req = PaymentIntentRequest::new(
            "portal",
            "INV-1001",
            "10.00",
            "Ahmad bin Abdullah",
            "ahmad@example.com",
        )
        .payment_channel(1);
        assert_eq!(
            create_payment_intent_checksum(SECRET, &req),
            "5d2619443b308ed20502e2530755f766c9b5f2a1df869cc343ec2b774025291b"
        );
    }

    #[test]
    fn payment_intent_multiple_channels_matches_php_vector() {
        let req = PaymentIntentRequest::new(
            "portal",
            "INV-1001",
            "10.00",
            "Ahmad bin Abdullah",
            "ahmad@example.com",
        )
        .payment_channel(vec![1, 5]);
        assert_eq!(
            create_payment_intent_checksum(SECRET, &req),
            "278a1f25a329755ef2bf66d91b77375557280350be74c7de354677e4ffede5f3"
        );
    }

    #[test]
    fn payment_intent_no_channel_matches_php_vector() {
        let req = PaymentIntentRequest::new(
            "portal",
            "INV-1001",
            "10.00",
            "Ahmad bin Abdullah",
            "ahmad@example.com",
        );
        assert_eq!(
            create_payment_intent_checksum(SECRET, &req),
            "7c46bb670deb36679cce862014e937393836b580a053e173e1053c6d05cd44c5"
        );
    }

    #[test]
    fn enrolment_matches_php_vector() {
        let req = FpxDirectDebitEnrolmentRequest {
            portal_key: "portal".into(),
            order_number: "DD-1001".into(),
            amount: "10.00".into(),
            payer_name: "Ahmad bin Abdullah".into(),
            payer_id_type: 1,
            payer_id: "900101011234".into(),
            payer_email: "ahmad@example.com".into(),
            payer_telephone_number: "0123456789".into(),
            application_reason: "Monthly subscription".into(),
            frequency_mode: "MT".into(),
            ..Default::default()
        };
        assert_eq!(
            create_fpx_direct_debit_enrolment_checksum(SECRET, &req),
            "40dcecefe94b010de207b110310d79a0bdaad9127c7e5fa46944d775ddbba4a8"
        );
    }

    #[test]
    fn maintenance_matches_php_vector() {
        let req = FpxDirectDebitMaintenanceRequest {
            amount: "15.00".into(),
            payer_email: "ahmad@example.com".into(),
            payer_telephone_number: "0123456789".into(),
            application_reason: "Update amount".into(),
            frequency_mode: "MT".into(),
            ..Default::default()
        };
        assert_eq!(
            create_fpx_direct_debit_maintenance_checksum(SECRET, &req),
            "833cac83f4d384a4800c1b64ce5de9824186475a450b18e187821cff3f1f53ed"
        );
    }

    #[test]
    fn generic_checksum_matches_php_vector() {
        let mut payload = BTreeMap::new();
        payload.insert("b".to_string(), "2".to_string());
        payload.insert("a".to_string(), "1".to_string());
        payload.insert("c".to_string(), "3".to_string());
        assert_eq!(
            create_checksum_value(SECRET, &payload),
            "0f47912baa8d5857541618c628c0b0f7e5d8b2a1e1d36a7d3b6a18da0af91066"
        );
    }
}
