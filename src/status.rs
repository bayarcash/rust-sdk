//! Status codes and human-readable helpers.
//!
//! Ports `Fpx`, `FpxDirectDebit` and `DuitNow\Dobw` from the PHP SDK, keeping
//! the same integer/string constants and the `getStatusText`-style helpers.

/// FPX transaction status codes and labels (port of PHP `Fpx`).
pub struct Fpx;

impl Fpx {
    /// New.
    pub const STATUS_NEW: i32 = 0;
    /// Pending.
    pub const STATUS_PENDING: i32 = 1;
    /// Failed.
    pub const STATUS_FAILED: i32 = 2;
    /// Successful.
    pub const STATUS_SUCCESS: i32 = 3;
    /// Cancelled.
    pub const STATUS_CANCELLED: i32 = 4;

    /// Human-readable label for a status code (`"UNKNOWN STATUS"` if unrecognized).
    pub fn status_text(status_code: i32) -> &'static str {
        match status_code {
            Self::STATUS_NEW => "New",
            Self::STATUS_PENDING => "Pending",
            Self::STATUS_CANCELLED => "Cancelled",
            Self::STATUS_SUCCESS => "Successful",
            Self::STATUS_FAILED => "Failed",
            _ => "UNKNOWN STATUS",
        }
    }
}

/// FPX Direct Debit constants and helpers (port of PHP `FpxDirectDebit`).
pub struct FpxDirectDebit;

impl FpxDirectDebit {
    // --- Application type ---
    /// Enrolment application type (`"01"`).
    pub const ENROLMENT: &'static str = "01";
    /// Maintenance application type (`"02"`).
    pub const MAINTENANCE: &'static str = "02";
    /// Termination application type (`"03"`).
    pub const TERMINATION: &'static str = "03";

    // --- Payer ID type ---
    /// New IC (NRIC).
    pub const NRIC: i32 = 1;
    /// Old IC.
    pub const OLD_IC: i32 = 2;
    /// Passport.
    pub const PASSPORT: i32 = 3;
    /// Business registration.
    pub const BUSINESS_REGISTRATION: i32 = 4;
    /// Others.
    pub const OTHERS: i32 = 5;

    // --- Frequency mode ---
    /// Daily (`"DL"`).
    pub const MODE_DAILY: &'static str = "DL";
    /// Weekly (`"WK"`).
    pub const MODE_WEEKLY: &'static str = "WK";
    /// Monthly (`"MT"`).
    pub const MODE_MONTHLY: &'static str = "MT";
    /// Yearly (`"YR"`).
    pub const MODE_YEARLY: &'static str = "YR";

    // --- Mandate status ---
    /// New.
    pub const STATUS_NEW: i32 = 0;
    /// Waiting approval.
    pub const STATUS_WAITING_APPROVAL: i32 = 1;
    /// Bank verification failed.
    pub const STATUS_FAILED_BANK_VERIFICATION: i32 = 2;
    /// Active.
    pub const STATUS_ACTIVE: i32 = 3;
    /// Terminated.
    pub const STATUS_TERMINATED: i32 = 4;
    /// Approved.
    pub const STATUS_APPROVED: i32 = 5;
    /// Rejected.
    pub const STATUS_REJECTED: i32 = 6;
    /// Cancelled.
    pub const STATUS_CANCELLED: i32 = 7;
    /// Error.
    pub const STATUS_ERROR: i32 = 8;

    /// Human-readable label for a mandate status code
    /// (`"UNKNOWN STATUS"` if unrecognized).
    pub fn status_text(status_code: i32) -> &'static str {
        match status_code {
            Self::STATUS_NEW => "New",
            Self::STATUS_WAITING_APPROVAL => "Waiting Approval",
            Self::STATUS_FAILED_BANK_VERIFICATION => "Bank Verification Failed",
            Self::STATUS_APPROVED => "Approved",
            Self::STATUS_REJECTED => "Rejected",
            Self::STATUS_CANCELLED => "Cancelled",
            Self::STATUS_ERROR => "Error",
            Self::STATUS_ACTIVE => "Active",
            Self::STATUS_TERMINATED => "Terminated",
            _ => "UNKNOWN STATUS",
        }
    }

    /// Label for an application type code, or `None` if unrecognized.
    pub fn application_type_text(application_type: &str) -> Option<&'static str> {
        match application_type {
            Self::ENROLMENT => Some("Enrollment"),
            Self::MAINTENANCE => Some("Maintenance"),
            Self::TERMINATION => Some("Termination"),
            _ => None,
        }
    }

    /// Label for a frequency-mode code, or `None` if unrecognized.
    pub fn frequency_mode_text(frequency_mode: &str) -> Option<&'static str> {
        match frequency_mode {
            Self::MODE_DAILY => Some("Daily"),
            Self::MODE_WEEKLY => Some("Weekly"),
            Self::MODE_MONTHLY => Some("Monthly"),
            Self::MODE_YEARLY => Some("Yearly"),
            _ => None,
        }
    }
}

/// DuitNow channels.
pub mod duitnow {
    /// DuitNow Online Banking / Wallet (DOBW) constants (port of PHP `DuitNow\Dobw`).
    pub struct Dobw;

    impl Dobw {
        /// Current/savings account.
        pub const CASA: &'static str = "01";
        /// Credit card.
        pub const CREDIT_CARD: &'static str = "02";
        /// eWallet.
        pub const EWALLET: &'static str = "03";

        /// New.
        pub const STATUS_NEW: i32 = 0;
        /// Pending.
        pub const STATUS_PENDING: i32 = 1;
        /// Failed.
        pub const STATUS_FAILED: i32 = 2;
        /// Successful.
        pub const STATUS_SUCCESS: i32 = 3;
        /// Cancelled.
        pub const STATUS_CANCELLED: i32 = 4;

        /// Human-readable label for a status code (`"UNKNOWN STATUS"` if unrecognized).
        pub fn status_text(status_code: i32) -> &'static str {
            match status_code {
                Self::STATUS_NEW => "New",
                Self::STATUS_PENDING => "Pending",
                Self::STATUS_CANCELLED => "Cancelled",
                Self::STATUS_SUCCESS => "Successful",
                Self::STATUS_FAILED => "Failed",
                _ => "UNKNOWN STATUS",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpx_status_text() {
        assert_eq!(Fpx::status_text(Fpx::STATUS_SUCCESS), "Successful");
        assert_eq!(Fpx::status_text(Fpx::STATUS_NEW), "New");
        assert_eq!(Fpx::status_text(99), "UNKNOWN STATUS");
    }

    #[test]
    fn direct_debit_helpers() {
        assert_eq!(FpxDirectDebit::status_text(FpxDirectDebit::STATUS_ACTIVE), "Active");
        assert_eq!(
            FpxDirectDebit::application_type_text(FpxDirectDebit::ENROLMENT),
            Some("Enrollment")
        );
        assert_eq!(FpxDirectDebit::application_type_text("99"), None);
        assert_eq!(
            FpxDirectDebit::frequency_mode_text(FpxDirectDebit::MODE_MONTHLY),
            Some("Monthly")
        );
    }

    #[test]
    fn dobw_status_text() {
        use duitnow::Dobw;
        assert_eq!(Dobw::status_text(Dobw::STATUS_SUCCESS), "Successful");
        assert_eq!(Dobw::CASA, "01");
    }
}
