use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Current state of the wallet subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalletState {
    /// Lightning feature is not compiled in.
    Disabled,
    /// Feature is enabled but wallet has not been initialized yet (no seed configured).
    Uninitialized,
    /// Wallet is currently starting up.
    Initializing,
    /// Wallet is ready to create invoices and receive payments.
    Ready,
    /// Wallet encountered an error during initialization.
    Error { message: String },
}

/// Wallet balance broken down by layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletBalance {
    /// Balance held in the trusted (Spark) wallet, in sats.
    pub trusted_sats: u64,
    /// Balance available on Lightning channels, in sats.
    pub lightning_sats: u64,
    /// Pending on-chain balance, in sats.
    pub pending_sats: u64,
    /// Total available balance (trusted + lightning), in sats.
    pub total_sats: u64,
}

/// A Lightning invoice ready for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Invoice {
    /// BOLT11 encoded invoice string.
    pub bolt11: String,
    /// QR code as an SVG string.
    pub qr_svg: String,
    /// Requested amount in sats (if specified).
    pub amount_sats: Option<u64>,
}

/// Request body for creating an invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInvoiceRequest {
    /// Amount in satoshis. If omitted, creates an amountless invoice.
    pub amount_sats: Option<u64>,
}

/// Notification that a payment was received.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentReceivedEvent {
    /// Amount received in millisatoshis.
    pub amount_msats: u64,
    /// Amount received in satoshis (amount_msats / 1000).
    pub amount_sats: u64,
    /// Hex-encoded payment hash.
    pub payment_hash: String,
}

/// Request body for paying a Lightning invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PayInvoiceRequest {
    /// BOLT11 invoice string to pay.
    pub bolt11: String,
}

/// Response after initiating a payment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PayInvoiceResponse {
    /// Whether the payment was successfully initiated.
    pub success: bool,
    /// Amount paid in sats.
    pub amount_sats: u64,
}

/// Request body for parsing a Lightning invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseInvoiceRequest {
    /// BOLT11 invoice string to parse.
    pub bolt11: String,
}

/// Parsed invoice details returned before payment confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParsedInvoice {
    /// Amount in sats (if the invoice specifies one).
    pub amount_sats: Option<u64>,
    /// Human-readable description from the invoice, if any.
    pub description: Option<String>,
}

/// Response for wallet status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletStatusResponse {
    pub state: WalletState,
}

#[cfg(feature = "lightning")]
mod orange;
#[cfg(not(feature = "lightning"))]
mod stub;

#[cfg(feature = "lightning")]
pub use self::orange::WalletManager;
#[cfg(not(feature = "lightning"))]
pub use self::stub::WalletManager;
