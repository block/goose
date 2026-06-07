//! Discovery of MCP servers from `mcp://` URIs per
//! [draft-serra-mcp-discovery-uri](https://datatracker.ietf.org/doc/draft-serra-mcp-discovery-uri/).
//!
//! Resolution order:
//! 1. (optional, "fast mode") DNS TXT `_mcp.{host}` for a hint, and
//!    `_mcp-key.{host}` for the public key used to verify manifest signatures.
//! 2. (authoritative) `GET https://{host}/.well-known/mcp-server`.
//! 3. (fallback) direct handshake probe at `https://{host}/mcp`.
//!
//! A `.well-known` manifest always takes precedence over DNS hints. All
//! endpoints must be HTTPS and the endpoint host must equal, or be a subdomain
//! of, the discovery host. When a manifest carries a signature it MUST verify.

mod dns;
mod error;
mod http;
mod jws;
mod manifest;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::agents::extension::ExtensionConfig;

pub use dns::{DnsJwk, DnsResolver, HickoryDnsResolver, McpDnsHint};
pub use error::{DiscoveryError, Result};
pub use http::{FetchOutcome, ManifestFetcher, ReqwestFetcher};
pub use manifest::{
    host_matches, AuthRequirements, ManifestSignature, McpServerManifest, TrustClass,
};

pub const WELL_KNOWN_PATH: &str = "/.well-known/mcp-server";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Which step of the resolution chain produced the final endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    WellKnown,
    DirectFallback,
}

/// Options controlling resolution behaviour.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Perform the optional DNS fast-mode step (also required to verify
    /// manifest signatures, since the public key is published in DNS).
    pub use_dns: bool,
    /// Reject manifests that do not carry a verifiable signature. A present
    /// signature is always verified regardless of this flag.
    pub require_signature: bool,
    pub timeout: Duration,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            use_dns: true,
            require_signature: false,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// A successfully discovered MCP server, ready to be turned into an
/// [`ExtensionConfig`] after the caller confirms the trust/auth posture.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoveredServer {
    /// Authority (host and optional port) the `mcp://` URI pointed at.
    pub discovery_host: String,
    /// Resolved HTTPS MCP endpoint.
    pub endpoint: String,
    pub manifest: McpServerManifest,
    pub source: DiscoverySource,
    pub signature_verified: bool,
    pub trust_class: TrustClass,
    /// True when a DNS `src` hint disagreed with the authoritative manifest
    /// endpoint (the manifest wins, but the conflict is surfaced).
    pub dns_conflict: bool,
}

impl DiscoveredServer {
    /// Build a `streamable_http` extension config pointing at the resolved
    /// endpoint.
    pub fn to_extension_config(&self, timeout: u64) -> ExtensionConfig {
        let name = if self.manifest.name.trim().is_empty() {
            self.discovery_host.clone()
        } else {
            self.manifest.name.clone()
        };
        let description = self
            .manifest
            .description
            .clone()
            .unwrap_or_else(|| format!("MCP server discovered at {}", self.discovery_host));
        ExtensionConfig::streamable_http(name, self.endpoint.clone(), description, timeout)
    }
}

/// Resolve an `mcp://` URI using the system DNS resolver and a real HTTP client.
pub async fn resolve(uri: &str) -> Result<DiscoveredServer> {
    let opts = DiscoveryOptions::default();
    let fetcher = ReqwestFetcher::new(opts.timeout).map_err(|e| DiscoveryError::Network {
        url: uri.to_string(),
        source: e,
    })?;
    let dns = if opts.use_dns {
        match HickoryDnsResolver::from_system() {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::debug!("DNS resolver unavailable, skipping fast-mode: {e}");
                None
            }
        }
    } else {
        None
    };
    let dns_ref = dns.as_ref().map(|d| d as &dyn DnsResolver);
    resolve_with(uri, dns_ref, &fetcher, &opts).await
}

struct ParsedUri {
    /// host without port, used for DNS labels and host-match
    host: String,
    /// authority (host[:port]) used to build https URLs
    authority: String,
}

fn parse_uri(uri: &str) -> Result<ParsedUri> {
    let parsed =
        url::Url::parse(uri).map_err(|e| DiscoveryError::InvalidUri(format!("{uri}: {e}")))?;
    if parsed.scheme() != "mcp" {
        return Err(DiscoveryError::InvalidUri(format!(
            "expected scheme \"mcp\", got {:?}",
            parsed.scheme()
        )));
    }
    let host = parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or_else(|| DiscoveryError::InvalidUri(format!("{uri}: missing host")))?
        .to_string();
    let authority = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.clone(),
    };
    Ok(ParsedUri { host, authority })
}

/// Resolve with injectable DNS and HTTP backends (used by tests).
pub async fn resolve_with(
    uri: &str,
    dns: Option<&dyn DnsResolver>,
    fetcher: &dyn ManifestFetcher,
    opts: &DiscoveryOptions,
) -> Result<DiscoveredServer> {
    let ParsedUri { host, authority } = parse_uri(uri)?;

    // Step 1: optional DNS fast-mode. Failures are non-fatal; absent keys only
    // matter if the manifest later claims a signature.
    let mut dns_hint: Option<McpDnsHint> = None;
    let mut jwks: Vec<DnsJwk> = Vec::new();
    if let Some(resolver) = dns {
        match resolver.txt_lookup(&format!("_mcp.{host}")).await {
            Ok(records) => dns_hint = dns::parse_mcp_hint(&records),
            Err(e) => tracing::debug!("_mcp.{host} TXT lookup failed: {e}"),
        }
        match resolver.txt_lookup(&format!("_mcp-key.{host}")).await {
            Ok(records) => jwks = dns::parse_jwks(&records),
            Err(e) => tracing::debug!("_mcp-key.{host} TXT lookup failed: {e}"),
        }
    }

    // Step 2: authoritative .well-known manifest.
    let well_known_url = format!("https://{authority}{WELL_KNOWN_PATH}");
    let outcome = fetcher.get(&well_known_url).await.unwrap_or_else(|e| {
        tracing::debug!("well-known fetch error for {well_known_url}: {e}");
        FetchOutcome::NotFound
    });

    if let FetchOutcome::Found { body } = outcome {
        return build_from_manifest(
            &host,
            &authority,
            &body,
            &well_known_url,
            &jwks,
            &dns_hint,
            opts,
        );
    }

    // Step 3: direct handshake fallback.
    let endpoint = format!("https://{authority}/mcp");
    let reachable = fetcher
        .probe(&endpoint)
        .await
        .map_err(|e| DiscoveryError::Network {
            url: endpoint.clone(),
            source: e,
        })?;
    if !reachable {
        return Err(DiscoveryError::NotFound(host));
    }
    if opts.require_signature {
        return Err(DiscoveryError::SignatureVerification(
            "direct-handshake fallback cannot be signed".to_string(),
        ));
    }
    let manifest = McpServerManifest {
        mcp_version: String::new(),
        name: host.clone(),
        endpoint: endpoint.clone(),
        transport: "http".to_string(),
        description: None,
        auth: None,
        capabilities: Vec::new(),
        trust_class: None,
        signature: None,
    };
    Ok(DiscoveredServer {
        discovery_host: authority,
        endpoint,
        trust_class: manifest.effective_trust_class(),
        manifest,
        source: DiscoverySource::DirectFallback,
        signature_verified: false,
        dns_conflict: false,
    })
}

fn build_from_manifest(
    host: &str,
    authority: &str,
    body: &str,
    url: &str,
    jwks: &[DnsJwk],
    dns_hint: &Option<McpDnsHint>,
    opts: &DiscoveryOptions,
) -> Result<DiscoveredServer> {
    let manifest: McpServerManifest =
        serde_json::from_str(body).map_err(|e| DiscoveryError::MalformedManifest {
            url: url.to_string(),
            reason: e.to_string(),
        })?;
    manifest.validate(url)?;

    let endpoint_url =
        url::Url::parse(&manifest.endpoint).map_err(|e| DiscoveryError::MalformedManifest {
            url: url.to_string(),
            reason: format!("invalid endpoint {:?}: {e}", manifest.endpoint),
        })?;
    if endpoint_url.scheme() != "https" {
        return Err(DiscoveryError::InsecureEndpoint(manifest.endpoint.clone()));
    }
    let endpoint_host = endpoint_url.host_str().unwrap_or_default();
    if !host_matches(host, endpoint_host) {
        return Err(DiscoveryError::EndpointHostMismatch {
            endpoint_host: endpoint_host.to_string(),
            discovery_host: host.to_string(),
        });
    }

    let signature_verified = match &manifest.signature {
        Some(sig) => {
            jws::verify_signature(body, sig, jwks)?;
            true
        }
        None => {
            if opts.require_signature {
                return Err(DiscoveryError::SignatureVerification(
                    "manifest is unsigned but a signature is required".to_string(),
                ));
            }
            false
        }
    };

    let dns_conflict = dns_hint
        .as_ref()
        .and_then(|h| h.src.as_deref())
        .map(|src| src != manifest.endpoint)
        .unwrap_or(false);
    if dns_conflict {
        tracing::warn!(
            "DNS src hint disagrees with .well-known endpoint for {host}; using manifest endpoint"
        );
    }

    Ok(DiscoveredServer {
        discovery_host: authority.to_string(),
        endpoint: manifest.endpoint.clone(),
        trust_class: manifest.effective_trust_class(),
        manifest,
        source: DiscoverySource::WellKnown,
        signature_verified,
        dns_conflict,
    })
}

#[cfg(test)]
mod tests;
