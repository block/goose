use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::*;

/// In-memory DNS resolver: maps a TXT label to its records.
#[derive(Default)]
struct MockDns {
    records: HashMap<String, Vec<String>>,
}

impl MockDns {
    fn with(mut self, name: &str, records: &[&str]) -> Self {
        self.records.insert(
            name.to_string(),
            records.iter().map(|s| s.to_string()).collect(),
        );
        self
    }
}

#[async_trait]
impl DnsResolver for MockDns {
    async fn txt_lookup(&self, name: &str) -> anyhow::Result<Vec<String>> {
        Ok(self.records.get(name).cloned().unwrap_or_default())
    }
}

/// In-memory HTTP fetcher: maps URL -> body for GET, and a set of reachable
/// endpoints for probe.
#[derive(Default)]
struct MockHttp {
    bodies: HashMap<String, String>,
    reachable: Vec<String>,
    probed: Mutex<Vec<String>>,
}

impl MockHttp {
    fn with_body(mut self, url: &str, body: &str) -> Self {
        self.bodies.insert(url.to_string(), body.to_string());
        self
    }
    fn reachable(mut self, url: &str) -> Self {
        self.reachable.push(url.to_string());
        self
    }
}

#[async_trait]
impl ManifestFetcher for MockHttp {
    async fn get(&self, url: &str) -> anyhow::Result<FetchOutcome> {
        match self.bodies.get(url) {
            Some(body) => Ok(FetchOutcome::Found { body: body.clone() }),
            None => Ok(FetchOutcome::NotFound),
        }
    }
    async fn probe(&self, url: &str) -> anyhow::Result<bool> {
        self.probed.lock().unwrap().push(url.to_string());
        Ok(self.reachable.iter().any(|u| u == url))
    }
}

fn default_opts() -> DiscoveryOptions {
    DiscoveryOptions::default()
}

#[tokio::test]
async fn rejects_non_mcp_scheme() {
    let http = MockHttp::default();
    let err = resolve_with("https://example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::InvalidUri(_)));
}

#[tokio::test]
async fn rejects_missing_host() {
    let http = MockHttp::default();
    let err = resolve_with("mcp:example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::InvalidUri(_)));
}

#[tokio::test]
async fn well_known_manifest_resolves() {
    let body = r#"{"mcp_version":"2025-06-18","name":"Example","endpoint":"https://example.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let server = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.source, DiscoverySource::WellKnown);
    assert_eq!(server.endpoint, "https://example.com/mcp");
    assert_eq!(server.trust_class, TrustClass::Public);
    assert!(!server.signature_verified);

    let config = server.to_extension_config(300);
    match config {
        ExtensionConfig::StreamableHttp {
            uri,
            name,
            description,
            ..
        } => {
            assert_eq!(uri, "https://example.com/mcp");
            // Name is derived from the discovery host, not the manifest's
            // self-declared name, to prevent key collisions/shadowing.
            assert_eq!(name, "example.com");
            assert!(description.contains("Example"));
        }
        other => panic!("expected StreamableHttp, got {other:?}"),
    }
}

#[tokio::test]
async fn subdomain_endpoint_is_allowed() {
    let body = r#"{"mcp_version":"1","name":"x","endpoint":"https://api.example.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let server = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.endpoint, "https://api.example.com/mcp");
}

#[tokio::test]
async fn endpoint_host_mismatch_is_rejected() {
    let body =
        r#"{"mcp_version":"1","name":"x","endpoint":"https://evil.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let err = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::EndpointHostMismatch { .. }));
}

#[tokio::test]
async fn insecure_endpoint_is_rejected() {
    let body =
        r#"{"mcp_version":"1","name":"x","endpoint":"http://example.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let err = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::InsecureEndpoint(_)));
}

#[tokio::test]
async fn malformed_manifest_is_rejected() {
    let http =
        MockHttp::default().with_body("https://example.com/.well-known/mcp-server", "not json");
    let err = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::MalformedManifest { .. }));
}

#[tokio::test]
async fn falls_back_to_direct_handshake() {
    let http = MockHttp::default().reachable("https://example.com/mcp");
    let server = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.source, DiscoverySource::DirectFallback);
    assert_eq!(server.endpoint, "https://example.com/mcp");
}

#[tokio::test]
async fn fallback_preserves_uri_path() {
    let http = MockHttp::default().reachable("https://example.com/custom");
    let server = resolve_with("mcp://example.com/custom", None, &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.source, DiscoverySource::DirectFallback);
    assert_eq!(server.endpoint, "https://example.com/custom");
}

#[tokio::test]
async fn fallback_honors_validated_dns_src() {
    // No well-known manifest, but DNS advertises a same-domain src endpoint.
    let http = MockHttp::default().reachable("https://api.example.com/mcp");
    let dns = MockDns::default().with(
        "_mcp.example.com",
        &["v=mcp1; src=https://api.example.com/mcp"],
    );
    let server = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.source, DiscoverySource::DirectFallback);
    assert_eq!(server.endpoint, "https://api.example.com/mcp");
}

#[tokio::test]
async fn fallback_ignores_cross_host_dns_src() {
    // A cross-host src (a spoofed DNS answer) must be ignored; discovery falls
    // back to the default /mcp on the discovery host instead.
    let http = MockHttp::default().reachable("https://example.com/mcp");
    let dns = MockDns::default().with("_mcp.example.com", &["v=mcp1; src=https://evil.com/mcp"]);
    let server = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.endpoint, "https://example.com/mcp");
}

#[tokio::test]
async fn no_server_anywhere_is_not_found() {
    let http = MockHttp::default();
    let err = resolve_with("mcp://example.com", None, &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::NotFound(_)));
}

#[tokio::test]
async fn port_is_carried_into_urls() {
    let body = r#"{"mcp_version":"1","name":"x","endpoint":"https://example.com:8080/mcp","transport":"http"}"#;
    let http =
        MockHttp::default().with_body("https://example.com:8080/.well-known/mcp-server", body);
    let server = resolve_with("mcp://example.com:8080", None, &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.endpoint, "https://example.com:8080/mcp");
}

#[tokio::test]
async fn signed_manifest_without_published_key_is_rejected() {
    let signed = r#"{"mcp_version":"1","name":"x","endpoint":"https://example.com/mcp","transport":"http","signature":{"alg":"ES256","kid":"unknown","value":"AAAA"}}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", signed);
    // DNS has no key record.
    let dns = MockDns::default();
    let err = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::SignatureVerification(_)));
}

#[tokio::test]
async fn require_signature_rejects_unsigned_manifest() {
    let body =
        r#"{"mcp_version":"1","name":"x","endpoint":"https://example.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let opts = DiscoveryOptions {
        require_signature: true,
        ..DiscoveryOptions::default()
    };
    let err = resolve_with("mcp://example.com", None, &http, &opts)
        .await
        .unwrap_err();
    assert!(matches!(err, DiscoveryError::SignatureVerification(_)));
}

#[tokio::test]
async fn dns_conflict_is_flagged_but_manifest_wins() {
    let body =
        r#"{"mcp_version":"1","name":"x","endpoint":"https://example.com/mcp","transport":"http"}"#;
    let http = MockHttp::default().with_body("https://example.com/.well-known/mcp-server", body);
    let dns = MockDns::default().with(
        "_mcp.example.com",
        &["v=mcp1; src=https://example.com/other"],
    );
    let server = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
        .await
        .unwrap();
    assert_eq!(server.endpoint, "https://example.com/mcp");
    assert!(server.dns_conflict);
}

/// Tests that actually run JWS crypto. They require a jsonwebtoken backend,
/// which is only compiled when a TLS feature is enabled (see the `rustls-tls` /
/// `native-tls` features). The workspace `cargo test` build enables one via
/// feature unification; an isolated `cargo test -p goose` does not, so these are
/// gated to avoid a missing-CryptoProvider panic.
#[cfg(any(feature = "rustls-tls", feature = "native-tls"))]
mod signed {
    use super::*;

    const TEST_PRIV_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgJTbjqK9zMfMXjse+\nl3Xh27JJ/L4Zcav0d38UTOBb4gKhRANCAASogbMv5zuQmZAXDiEf3gvU7Nejtvdz\n86jQnBIrsFjqvIhIeOr5NvzOoE17sxE0eptq4XutZCpz32Ji5+GDY/L3\n-----END PRIVATE KEY-----\n";
    const TEST_JWK_X: &str = "qIGzL-c7kJmQFw4hH94L1OzXo7b3c_Oo0JwSK7BY6rw";
    const TEST_JWK_Y: &str = "iEh46vk2_M6gTXuzETR6m2rhe61kKnPfYmLn4YNj8vc";

    fn test_jwk_record(kid: &str) -> String {
        let jwk = format!(
            r#"{{"kty":"EC","crv":"P-256","x":"{TEST_JWK_X}","y":"{TEST_JWK_Y}","kid":"{kid}"}}"#
        );
        format!("v=mcp1jwk; kid={kid}; jwk={jwk}")
    }

    fn sign_canonical(unsigned_manifest: &str) -> String {
        let payload = crate::mcp_discovery::jws::canonical_payload(unsigned_manifest).unwrap();
        let key = jsonwebtoken::EncodingKey::from_ec_pem(TEST_PRIV_PEM.as_bytes()).unwrap();
        jsonwebtoken::crypto::sign(&payload, &key, jsonwebtoken::Algorithm::ES256).unwrap()
    }

    #[tokio::test]
    async fn valid_signature_verifies() {
        let unsigned = r#"{"mcp_version":"2025-06-18","name":"Signed","endpoint":"https://example.com/mcp","transport":"http"}"#;
        let sig = sign_canonical(unsigned);
        let signed = format!(
            r#"{{"mcp_version":"2025-06-18","name":"Signed","endpoint":"https://example.com/mcp","transport":"http","signature":{{"alg":"ES256","kid":"mcp-key-1","value":"{sig}"}}}}"#
        );
        let http =
            MockHttp::default().with_body("https://example.com/.well-known/mcp-server", &signed);
        let dns = MockDns::default().with("_mcp-key.example.com", &[&test_jwk_record("mcp-key-1")]);

        let server = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
            .await
            .unwrap();
        assert!(server.signature_verified);
    }

    #[tokio::test]
    async fn tampered_manifest_is_rejected() {
        let unsigned = r#"{"mcp_version":"1","name":"x","endpoint":"https://example.com/mcp","transport":"http"}"#;
        let sig = sign_canonical(unsigned);
        // Serve a manifest whose name differs from what was signed.
        let signed = format!(
            r#"{{"mcp_version":"1","name":"TAMPERED","endpoint":"https://example.com/mcp","transport":"http","signature":{{"alg":"ES256","kid":"mcp-key-1","value":"{sig}"}}}}"#
        );
        let http =
            MockHttp::default().with_body("https://example.com/.well-known/mcp-server", &signed);
        let dns = MockDns::default().with("_mcp-key.example.com", &[&test_jwk_record("mcp-key-1")]);

        let err = resolve_with("mcp://example.com", Some(&dns), &http, &default_opts())
            .await
            .unwrap_err();
        assert!(matches!(err, DiscoveryError::SignatureVerification(_)));
    }
}
