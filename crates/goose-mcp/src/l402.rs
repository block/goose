//! Global L402 payment callback for MCP tools.
//!
//! goose-server registers a callback at startup; MCP tools call it to pay invoices.

use once_cell::sync::OnceCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Async function that pays a BOLT11 invoice and returns the preimage hex string.
pub type PayInvoiceFn = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> + Send + Sync,
>;

static PAY_FN: OnceCell<PayInvoiceFn> = OnceCell::new();

/// Register the global L402 pay function. Called once by goose-server at startup.
pub fn set_l402_pay_fn(f: PayInvoiceFn) {
    let _ = PAY_FN.set(f);
}

/// Get the global L402 pay function, if registered.
pub fn get_l402_pay_fn() -> Option<&'static PayInvoiceFn> {
    PAY_FN.get()
}
