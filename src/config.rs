//! Configuration primitives.

/// The Bayarcash API version.
///
/// `V2` is the default (matching the PHP SDK). `V3` unlocks the additional
/// transaction query helpers and payment-intent retrieval/cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ApiVersion {
    /// API v2 (default).
    #[default]
    V2,
    /// API v3.
    V3,
}

impl ApiVersion {
    /// The string form used by the gateway (`"v2"` / `"v3"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ApiVersion::V2 => "v2",
            ApiVersion::V3 => "v3",
        }
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
