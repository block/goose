use anyhow::Context;
use async_trait::async_trait;
use jsonwebtoken::jwk::Jwk;

/// Hint extracted from the `_mcp.{host}` TXT record (the DNS "fast mode").
///
/// Per the draft this is advisory only: a `.well-known` manifest, when present,
/// always takes precedence over these values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpDnsHint {
    pub src: Option<String>,
    pub registry: Option<String>,
    pub auth: Option<String>,
}

/// A public key published via the `_mcp-key.{host}` TXT record, used to verify
/// a manifest's detached JWS signature.
#[derive(Debug, Clone)]
pub struct DnsJwk {
    pub kid: String,
    pub jwk: Jwk,
}

/// Abstraction over DNS TXT lookups so the resolver can be unit tested without
/// a network. Returns one `String` per TXT record (its data chunks joined).
#[async_trait]
pub trait DnsResolver: Send + Sync {
    async fn txt_lookup(&self, name: &str) -> anyhow::Result<Vec<String>>;
}

/// Real resolver backed by hickory using the host's `/etc/resolv.conf`.
pub struct HickoryDnsResolver {
    inner: hickory_resolver::TokioResolver,
}

impl HickoryDnsResolver {
    pub fn from_system() -> anyhow::Result<Self> {
        let inner = hickory_resolver::TokioResolver::builder_tokio()
            .context("building system DNS resolver")?
            .build()
            .context("building system DNS resolver")?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl DnsResolver for HickoryDnsResolver {
    async fn txt_lookup(&self, name: &str) -> anyhow::Result<Vec<String>> {
        use hickory_resolver::proto::rr::RData;

        let lookup = match self.inner.txt_lookup(name).await {
            Ok(lookup) => lookup,
            // A missing record is a normal, non-fatal outcome for an optional step.
            Err(e) if e.is_no_records_found() => return Ok(Vec::new()),
            Err(e) => return Err(e).context("TXT lookup failed"),
        };

        let records = lookup
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::TXT(txt) => Some(txt.to_string()),
                _ => None,
            })
            .collect();
        Ok(records)
    }
}

fn parse_pairs(record: &str) -> Vec<(String, String)> {
    record
        .split(';')
        .filter_map(|segment| {
            let segment = segment.trim();
            if segment.is_empty() {
                return None;
            }
            let (key, value) = segment.split_once('=')?;
            Some((key.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

/// Parse the first valid `v=mcp1` discovery record. `endpoint=` is accepted as a
/// deprecated alias for `src=`.
pub fn parse_mcp_hint(records: &[String]) -> Option<McpDnsHint> {
    for record in records {
        let pairs = parse_pairs(record);
        let version = pairs.iter().find(|(k, _)| k == "v").map(|(_, v)| v);
        if version.map(String::as_str) != Some("mcp1") {
            continue;
        }
        let get = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        return Some(McpDnsHint {
            src: get("src").or_else(|| get("endpoint")),
            registry: get("registry"),
            auth: get("auth"),
        });
    }
    None
}

/// Parse every valid `v=mcp1jwk` key record. Malformed JWKs are skipped.
pub fn parse_jwks(records: &[String]) -> Vec<DnsJwk> {
    let mut keys = Vec::new();
    for record in records {
        let pairs = parse_pairs(record);
        if pairs
            .iter()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.as_str())
            != Some("mcp1jwk")
        {
            continue;
        }
        let kid = pairs
            .iter()
            .find(|(k, _)| k == "kid")
            .map(|(_, v)| v.clone());
        let jwk_raw = pairs
            .iter()
            .find(|(k, _)| k == "jwk")
            .map(|(_, v)| v.clone());
        if let (Some(kid), Some(jwk_raw)) = (kid, jwk_raw) {
            if let Ok(jwk) = serde_json::from_str::<Jwk>(&jwk_raw) {
                keys.push(DnsJwk { kid, jwk });
            }
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_src_hint() {
        let hint =
            parse_mcp_hint(&["v=mcp1; src=https://api.example.com/mcp".to_string()]).unwrap();
        assert_eq!(hint.src.as_deref(), Some("https://api.example.com/mcp"));
        assert_eq!(hint.registry, None);
    }

    #[test]
    fn endpoint_is_alias_for_src() {
        let hint =
            parse_mcp_hint(&["v=mcp1; endpoint=https://example.com/mcp; auth=oauth2".to_string()])
                .unwrap();
        assert_eq!(hint.src.as_deref(), Some("https://example.com/mcp"));
        assert_eq!(hint.auth.as_deref(), Some("oauth2"));
    }

    #[test]
    fn ignores_non_mcp_txt_records() {
        let records = vec![
            "v=spf1 include:_spf.example.com ~all".to_string(),
            "v=mcp1; registry=https://registry.example.com".to_string(),
        ];
        let hint = parse_mcp_hint(&records).unwrap();
        assert_eq!(
            hint.registry.as_deref(),
            Some("https://registry.example.com")
        );
        assert_eq!(hint.src, None);
    }

    #[test]
    fn jwk_record_without_jwk_is_skipped() {
        assert!(parse_jwks(&["v=mcp1jwk; kid=mcp-key-1".to_string()]).is_empty());
    }
}
