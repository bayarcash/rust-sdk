//! # Bayarcash Rust SDK
//!
//! An idiomatic, async Rust client for the [Bayarcash](https://bayarcash.com)
//! payment gateway — a feature-parity port of the official Bayarcash PHP SDK.
//! It supports API **v2** (default) and **v3**, and is byte-compatible with the
//! gateway for checksum generation and callback verification.
//!
//! ## Quick start
//!
//! ```no_run
//! use bayarcash::{Bayarcash, PaymentIntentRequest};
//!
//! # async fn run() -> Result<(), bayarcash::Error> {
//! let client = Bayarcash::new("YOUR_API_TOKEN").use_sandbox();
//! let secret = "YOUR_API_SECRET_KEY";
//!
//! let request = PaymentIntentRequest::new(
//!         "your_portal_key",
//!         "INV-1001",
//!         "10.00",
//!         "Ahmad bin Abdullah",
//!         "ahmad@example.com",
//!     )
//!     .payment_channel(Bayarcash::FPX)
//!     .return_url("https://your-site.com/payment/return")
//!     .callback_url("https://your-site.com/payment/callback")
//!     .signed(secret); // append the checksum
//!
//! let intent = client.create_payment_intent(&request).await?;
//! // redirect the payer to `intent.url`
//! # Ok(())
//! # }
//! ```
//!
//! ## Verifying callbacks
//!
//! ```
//! use bayarcash::Bayarcash;
//! use std::collections::HashMap;
//!
//! let client = Bayarcash::new("token");
//! let callback: HashMap<String, String> = HashMap::new(); // e.g. $_POST / request.all()
//! if client.verify_transaction_callback_data(&callback, "YOUR_API_SECRET_KEY") {
//!     // data is authentic — safe to process
//! }
//! ```
#![warn(missing_docs)]

mod callback;
pub mod channels;
pub mod checksum;
mod client;
mod config;
mod error;
mod manual_transfer;
mod models;
mod requests;
mod status;

pub use callback::{
    verify_direct_debit_authorization_callback_data,
    verify_direct_debit_bank_approval_callback_data, verify_direct_debit_transaction_callback_data,
    verify_pre_transaction_callback_data, verify_return_url_callback_data,
    verify_transaction_callback_data, CallbackData,
};
pub use client::Bayarcash;
pub use config::ApiVersion;
pub use error::{Error, Result, ValidationErrors};
pub use models::{
    FpxBank, Mandate, MandateApplication, ManualBankTransferResponse, PaymentIntent, Portal,
    Transaction, TransactionList,
};
pub use requests::{
    FpxDirectDebitEnrolmentRequest, FpxDirectDebitMaintenanceRequest,
    FpxDirectDebitTerminationRequest, ManualBankTransferRequest, PaymentChannel,
    PaymentIntentRequest, TransactionFilters,
};
pub use status::{duitnow, Fpx, FpxDirectDebit};
