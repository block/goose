//! L402 (Lightning HTTP 402) protocol support.
//!
//! When an HTTP API returns 402 with a `WWW-Authenticate: L402 macaroon="...", invoice="lnbc..."`
//! header, we automatically pay the Lightning invoice, get the preimage, and retry the request
//! with `Authorization: L402 <macaroon>:<preimage>`.

use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Handler that can pay a BOLT11 Lightning invoice and return the preimage.
#[async_trait]
pub trait L402PaymentHandler: Send + Sync {
    /// Pay the given BOLT11 invoice and return the payment preimage as a hex string.
    async fn pay_invoice(&self, bolt11: &str) -> anyhow::Result<String>;
}

/// Global L402 payment handler, set once by goose-server when a wallet is available.
static L402_HANDLER: OnceCell<Arc<dyn L402PaymentHandler>> = OnceCell::new();

/// Register the global L402 payment handler.
/// Returns `Err` if a handler was already registered.
pub fn set_l402_handler(
    handler: Arc<dyn L402PaymentHandler>,
) -> Result<(), Arc<dyn L402PaymentHandler>> {
    L402_HANDLER.set(handler)
}

/// Get the global L402 payment handler, if one has been registered.
pub fn get_l402_handler() -> Option<&'static Arc<dyn L402PaymentHandler>> {
    L402_HANDLER.get()
}

/// A parsed L402 challenge from a `WWW-Authenticate` header.
#[derive(Debug, Clone)]
pub struct L402Challenge {
    /// The macaroon token from the challenge.
    pub macaroon: String,
    /// The BOLT11 Lightning invoice to pay.
    pub invoice: String,
}

/// Parse an L402 challenge from a `WWW-Authenticate` header value.
///
/// Expected format: `L402 macaroon="<macaroon>", invoice="<invoice>"`
pub fn parse_l402_challenge(header_value: &str) -> Option<L402Challenge> {
    let header_value = header_value.trim();

    // Must start with "L402 " (case-insensitive).
    if !header_value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("L402 "))
    {
        return None;
    }

    let params = header_value.get(5..).unwrap_or("");

    let macaroon = extract_quoted_param(params, "macaroon")?;
    let invoice = extract_quoted_param(params, "invoice")?;

    Some(L402Challenge { macaroon, invoice })
}

/// Extract a quoted parameter value like `key="value"` from a parameter string.
fn extract_quoted_param(params: &str, key: &str) -> Option<String> {
    // Look for key="
    let search = format!("{key}=\"");
    let start = params.find(&search)?;
    let value_start = start + search.len();
    let rest = params.get(value_start..)?;
    let end = rest.find('"')?;
    Some(rest.get(..end)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_l402_challenge() {
        let header = r#"L402 macaroon="abc123", invoice="lnbc10n1pj...""#;
        let challenge = parse_l402_challenge(header).unwrap();
        assert_eq!(challenge.macaroon, "abc123");
        assert_eq!(challenge.invoice, "lnbc10n1pj...");
    }

    #[test]
    fn test_parse_l402_challenge_case_insensitive() {
        let header = r#"l402 macaroon="mac", invoice="inv""#;
        let challenge = parse_l402_challenge(header).unwrap();
        assert_eq!(challenge.macaroon, "mac");
        assert_eq!(challenge.invoice, "inv");
    }

    #[test]
    fn test_parse_l402_challenge_missing_macaroon() {
        let header = r#"L402 invoice="lnbc...""#;
        assert!(parse_l402_challenge(header).is_none());
    }

    #[test]
    fn test_parse_l402_challenge_missing_invoice() {
        let header = r#"L402 macaroon="abc""#;
        assert!(parse_l402_challenge(header).is_none());
    }

    #[test]
    fn test_parse_l402_challenge_wrong_scheme() {
        let header = r#"Bearer token="abc""#;
        assert!(parse_l402_challenge(header).is_none());
    }

    #[test]
    fn test_parse_l402_challenge_reversed_order() {
        let header = r#"L402 invoice="lnbc10n1pj...", macaroon="abc123""#;
        let challenge = parse_l402_challenge(header).unwrap();
        assert_eq!(challenge.macaroon, "abc123");
        assert_eq!(challenge.invoice, "lnbc10n1pj...");
    }

    /// Hit a real L402 endpoint on mainnet and verify we can parse the challenge.
    /// Ignored by default — run with: cargo test -p goose l402_live -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_l402_live_challenge_parsing() {
        let response = reqwest::get("https://api.myceliasignal.com/oracle/btcusd")
            .await
            .expect("request failed");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::PAYMENT_REQUIRED,
            "expected 402 from L402 endpoint"
        );

        let www_auth = response
            .headers()
            .get("www-authenticate")
            .expect("missing WWW-Authenticate header")
            .to_str()
            .expect("header not valid UTF-8");

        let challenge = parse_l402_challenge(www_auth)
            .expect("failed to parse L402 challenge from live endpoint");

        assert!(
            !challenge.macaroon.is_empty(),
            "macaroon should not be empty"
        );
        assert!(
            challenge.invoice.starts_with("lnbc"),
            "invoice should be a mainnet BOLT11: {}",
            challenge.invoice
        );

        println!("L402 challenge parsed successfully:");
        println!(
            "  macaroon: {}...{}",
            challenge.macaroon.get(..20).unwrap_or(&challenge.macaroon),
            challenge
                .macaroon
                .get(challenge.macaroon.len().saturating_sub(10)..)
                .unwrap_or(&challenge.macaroon),
        );
        println!(
            "  invoice:  {}...",
            challenge.invoice.get(..40).unwrap_or(&challenge.invoice)
        );
    }

    /// Full L402 round-trip: hit endpoint, pay the invoice, verify we get data back.
    /// Requires a funded Lightning wallet. Run with:
    ///   cargo test -p goose l402_live_pay -- --ignored
    /// The wallet's L402 handler must be registered before running.
    #[tokio::test]
    #[ignore]
    async fn test_l402_live_pay_round_trip() {
        use crate::providers::api_client::{ApiClient, AuthMethod};

        assert!(
            get_l402_handler().is_some(),
            "No L402 handler registered — start goose-server with a funded wallet first"
        );

        let client = ApiClient::new(
            "https://api.myceliasignal.com".to_string(),
            AuthMethod::NoAuth,
        )
        .expect("failed to create API client");

        let response = client
            .response_get(None, "oracle/btcusd")
            .await
            .expect("request failed");

        assert!(
            response.status().is_success(),
            "expected 200 after L402 payment, got {}",
            response.status()
        );

        let body = response.text().await.expect("failed to read body");
        assert!(!body.is_empty(), "response body should not be empty");
        println!("L402 round-trip successful! Response:\n{body}");
    }
}
