//! Typed error handling for the Bayarcash SDK.
//!
//! Mirrors the PHP SDK's exception mapping in `MakesHttpRequests::handleRequestError`:
//!
//! | HTTP | Variant |
//! |------|---------|
//! | 422  | [`Error::Validation`] |
//! | 404  | [`Error::NotFound`] |
//! | 400  | [`Error::FailedAction`] |
//! | 429  | [`Error::RateLimitExceeded`] |
//! | other | [`Error::Api`] |

use std::collections::HashMap;

use thiserror::Error;

/// Convenience result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Validation payload extracted from a `422 Unprocessable Entity` response.
///
/// Mirrors the data carried by the PHP `ValidationException`. Laravel-style
/// validation responses look like:
///
/// ```json
/// { "message": "The given data was invalid.", "errors": { "amount": ["The amount field is required."] } }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationErrors {
    /// Top-level `message`, if present.
    pub message: Option<String>,
    /// Per-field validation messages, keyed by field name.
    pub errors: HashMap<String, Vec<String>>,
    /// The raw decoded JSON body, for anything not captured above.
    pub raw: serde_json::Value,
}

impl ValidationErrors {
    /// The per-field validation errors (mirrors `$e->errors()` in PHP).
    pub fn errors(&self) -> &HashMap<String, Vec<String>> {
        &self.errors
    }

    pub(crate) fn from_value(value: serde_json::Value) -> Self {
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let mut errors = HashMap::new();
        if let Some(map) = value.get("errors").and_then(|e| e.as_object()) {
            for (field, msgs) in map {
                let list = match msgs {
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .collect(),
                    serde_json::Value::String(s) => vec![s.clone()],
                    other => vec![other.to_string()],
                };
                errors.insert(field.clone(), list);
            }
        }

        ValidationErrors {
            message,
            errors,
            raw: value,
        }
    }
}

/// All errors surfaced by the SDK.
#[derive(Debug, Error)]
pub enum Error {
    /// `422` — the request data failed validation. Inspect [`ValidationErrors::errors`].
    #[error("validation failed{}", .0.message.as_deref().map(|m| format!(": {m}")).unwrap_or_default())]
    Validation(ValidationErrors),

    /// `404` — the requested resource does not exist.
    #[error("resource not found")]
    NotFound,

    /// `400` — the request could not be completed. Carries the gateway message.
    #[error("failed action: {0}")]
    FailedAction(String),

    /// `429` — too many requests. `resets_at` is the `x-ratelimit-reset` value, if sent.
    #[error("rate limit exceeded")]
    RateLimitExceeded {
        /// Unix timestamp at which the limit resets, from `x-ratelimit-reset`.
        resets_at: Option<i64>,
    },

    /// Any other non-2xx response, with the raw status and body.
    #[error("API error (status {status}): {body}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Raw response body.
        body: String,
    },

    /// A required argument was missing or invalid (mirrors PHP `InvalidArgumentException`).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The operation is not available for the configured API version
    /// (e.g. a v3-only query invoked on v2).
    #[error("{0}")]
    Unsupported(String),

    /// Transport-level failure from `reqwest`.
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// JSON (de)serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Local I/O failure (e.g. reading a proof-of-payment file).
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
