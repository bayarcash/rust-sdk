//! Payment-channel identifiers.
//!
//! These mirror the `Bayarcash::*` class constants in the PHP SDK exactly,
//! including the historical gap at id `20` (there is no channel `20`;
//! [`SHOPEE_PAY`] is `21`). The same values are also exposed as associated
//! constants on [`crate::Bayarcash`] (e.g. `Bayarcash::FPX`).

/// FPX Online Banking.
pub const FPX: i32 = 1;
/// Manual Bank Transfer.
pub const MANUAL_TRANSFER: i32 = 2;
/// FPX Direct Debit.
pub const FPX_DIRECT_DEBIT: i32 = 3;
/// FPX Line of Credit.
pub const FPX_LINE_OF_CREDIT: i32 = 4;
/// DuitNow Online Banking / Wallets (DOBW).
pub const DUITNOW_DOBW: i32 = 5;
/// DuitNow QR.
pub const DUITNOW_QR: i32 = 6;
/// ShopeePayLater (SPayLater).
pub const SPAYLATER: i32 = 7;
/// Boost PayFlex.
pub const BOOST_PAYFLEX: i32 = 8;
/// QRIS Online Banking.
pub const QRISOB: i32 = 9;
/// QRIS Wallet.
pub const QRISWALLET: i32 = 10;
/// NETS.
pub const NETS: i32 = 11;
/// Credit Card.
pub const CREDIT_CARD: i32 = 12;
/// Alipay.
pub const ALIPAY: i32 = 13;
/// WeChat Pay.
pub const WECHATPAY: i32 = 14;
/// PromptPay.
pub const PROMPTPAY: i32 = 15;
/// Touch 'n Go eWallet.
pub const TOUCH_N_GO: i32 = 16;
/// Boost Wallet.
pub const BOOST_WALLET: i32 = 17;
/// GrabPay.
pub const GRABPAY: i32 = 18;
/// Grab PayLater.
pub const GRABPL: i32 = 19;
// NOTE: there is no channel id 20 — the gap is intentional and mirrors the gateway.
/// ShopeePay.
pub const SHOPEE_PAY: i32 = 21;
