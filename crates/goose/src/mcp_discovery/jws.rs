use jsonwebtoken::{Algorithm, DecodingKey};
use serde_json::Value;

use super::dns::DnsJwk;
use super::error::DiscoveryError;
use super::manifest::ManifestSignature;

/// Verify a manifest's detached JWS signature against the public keys published
/// in DNS.
///
/// The signature covers the canonical JSON serialization of the manifest with
/// the `signature` field removed (draft-serra-mcp-discovery-uri, Security
/// Considerations). The signed value is a raw base64url signature carried in
/// `signature.value`, with `alg`/`kid` selecting the algorithm and key.
pub fn verify_signature(
    raw_manifest_json: &str,
    sig: &ManifestSignature,
    jwks: &[DnsJwk],
) -> Result<(), DiscoveryError> {
    let key_entry = jwks.iter().find(|k| k.kid == sig.kid).ok_or_else(|| {
        DiscoveryError::SignatureVerification(format!("no published key matches kid {:?}", sig.kid))
    })?;

    let algorithm = parse_alg(&sig.alg)?;

    let decoding_key = DecodingKey::from_jwk(&key_entry.jwk).map_err(|e| {
        DiscoveryError::SignatureVerification(format!("invalid JWK for kid {:?}: {e}", sig.kid))
    })?;

    let payload = canonical_payload(raw_manifest_json)?;

    let verified = jsonwebtoken::crypto::verify(&sig.value, &payload, &decoding_key, algorithm)
        .map_err(|e| DiscoveryError::SignatureVerification(format!("verification error: {e}")))?;

    if !verified {
        return Err(DiscoveryError::SignatureVerification(
            "signature does not match manifest".to_string(),
        ));
    }
    Ok(())
}

fn parse_alg(alg: &str) -> Result<Algorithm, DiscoveryError> {
    let parsed = match alg {
        "RS256" => Algorithm::RS256,
        "RS384" => Algorithm::RS384,
        "RS512" => Algorithm::RS512,
        "PS256" => Algorithm::PS256,
        "PS384" => Algorithm::PS384,
        "PS512" => Algorithm::PS512,
        "ES256" => Algorithm::ES256,
        "ES384" => Algorithm::ES384,
        "EdDSA" => Algorithm::EdDSA,
        // Symmetric algorithms cannot be published as a public key; reject.
        other => {
            return Err(DiscoveryError::SignatureVerification(format!(
                "unsupported signature algorithm {other:?}"
            )))
        }
    };
    Ok(parsed)
}

/// Produce the canonical byte payload that the signature is computed over: the
/// manifest object with `signature` removed and object keys sorted.
pub(crate) fn canonical_payload(raw_manifest_json: &str) -> Result<Vec<u8>, DiscoveryError> {
    let mut value: Value = serde_json::from_str(raw_manifest_json).map_err(|e| {
        DiscoveryError::SignatureVerification(format!("manifest is not valid JSON: {e}"))
    })?;
    if let Value::Object(map) = &mut value {
        map.remove("signature");
    }
    serde_json::to_vec(&canonicalize(&value)).map_err(|e| {
        DiscoveryError::SignatureVerification(format!("failed to canonicalize manifest: {e}"))
    })
}

/// Recursively sort object keys so serialization is deterministic regardless of
/// whether serde_json preserves insertion order in this build.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: std::collections::BTreeMap<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_payload_strips_signature_and_sorts_keys() {
        let raw = r#"{"name":"x","endpoint":"https://e","signature":{"alg":"ES256","kid":"k","value":"v"}}"#;
        let bytes = canonical_payload(raw).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("signature"));
        // keys sorted: endpoint before name
        assert!(text.find("endpoint").unwrap() < text.find("name").unwrap());
    }

    #[test]
    fn canonical_payload_is_order_independent() {
        let a = r#"{"a":1,"b":2}"#;
        let b = r#"{"b":2,"a":1}"#;
        assert_eq!(canonical_payload(a).unwrap(), canonical_payload(b).unwrap());
    }

    #[test]
    fn rejects_symmetric_algorithm() {
        assert!(parse_alg("HS256").is_err());
    }

    #[test]
    fn missing_key_for_kid_fails() {
        let sig = ManifestSignature {
            alg: "ES256".into(),
            kid: "absent".into(),
            value: "AAAA".into(),
        };
        let err = verify_signature("{}", &sig, &[]).unwrap_err();
        assert!(matches!(err, DiscoveryError::SignatureVerification(_)));
    }
}
