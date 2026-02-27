use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod approval;
pub use approval::PaymentApprovalManager;

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
    /// Amount in satoshis (required for amountless invoices, Lightning addresses, etc.).
    pub amount_sats: Option<u64>,
}

/// Response after initiating a payment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PayInvoiceResponse {
    /// Whether the payment was successfully initiated.
    pub success: bool,
    /// Amount paid in sats.
    pub amount_sats: u64,
    /// Payment preimage as a hex string (proof of payment).
    pub preimage: String,
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

/// Where an automatic payment originates from.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentSource {
    /// HTTP 402 L402 challenge auto-pay.
    L402Auto,
    /// MCP tool `pay_l402_invoice`.
    AgentTool,
}

/// A payment that needs user approval before proceeding.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentApprovalRequest {
    /// Unique ID for this approval request.
    pub id: String,
    /// BOLT11 invoice string.
    pub bolt11: String,
    /// Amount in satoshis (if known).
    pub amount_sats: Option<u64>,
    /// Human-readable description, if any.
    pub description: Option<String>,
    /// Where this payment request originated.
    pub source: PaymentSource,
    /// When this request was created (unix timestamp).
    pub created_at: u64,
    /// When this request expires (unix timestamp).
    pub expires_at: u64,
}

/// User response to a payment approval request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentApprovalResponse {
    /// The approval request ID being responded to.
    pub id: String,
    /// Whether the user approved the payment.
    pub approved: bool,
}

/// Direction of a wallet payment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentDirection {
    /// Incoming payment (received).
    Incoming,
    /// Outgoing payment (sent).
    Outgoing,
}

/// Status of a payment record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Completed,
}

/// A single payment record for display in the history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentRecord {
    /// Direction of the payment.
    pub direction: PaymentDirection,
    /// Status of the payment.
    pub status: PaymentStatus,
    /// Amount in satoshis.
    pub amount_sats: u64,
    /// Payment hash or txid.
    pub payment_hash: String,
    /// Unix timestamp when this payment was recorded.
    pub timestamp: u64,
    /// Human-readable description, if available.
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
