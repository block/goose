//! Signed invite / capability tokens.
//!
//! An invite carries everything a client needs to *find and connect to* a
//! roaming agent (the host's [`EndpointId`] plus relay URLs), together with a
//! capability scope, validity window, and a signature by the host's node key.
//!
//! Design notes (informed by a sibling production iroh project's bootstrap
//! token and an independent security review):
//!
//! * The signature covers **canonical bytes**, not a re-serialized JSON blob,
//!   with a domain-separation prefix so a signature can never be confused with
//!   another protocol's.
//! * We deliberately **do not** embed relay auth credentials or private/LAN
//!   direct addresses. Only the [`EndpointId`] and relay URLs travel; iroh
//!   upgrades to a direct path via hole-punching after connecting.
//! * `audience` binds the token to the issuing host's own id, and the optional
//!   `allowed_client_keys` binds redemption to specific client keys so a leaked
//!   bearer token cannot be redeemed by an arbitrary third party.

use base64::Engine;
use iroh::{EndpointAddr, EndpointId, PublicKey, SecretKey, Signature, TransportAddr};
use serde::{Deserialize, Serialize};

use crate::error::RoamingError;

const TOKEN_DOMAIN: &[u8] = b"goose-roaming-invite-v1";
const TOKEN_VERSION: u32 = 1;

/// What a redeemed invite lets the client do against the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Full ACP control: create/drive sessions and answer tool-permission
    /// prompts. This is effectively remote shell access to the host and should
    /// only be granted to trusted peers.
    Control,
    /// Attach to an existing live session and control it.
    Attach,
    /// Observe session activity without the ability to approve tool
    /// permissions or mutate state.
    Observe,
}

/// The signed body of an invite. All fields are covered by the signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteClaims {
    pub version: u32,
    /// Issuing host's endpoint id (== its node public key). Also the connect target.
    pub audience: EndpointId,
    /// Relay URLs where the host can be reached. Serialized as strings.
    pub relay_urls: Vec<String>,
    /// Capability granted by this invite.
    pub scope: Scope,
    /// Optional allowlist: only these client keys may redeem the invite.
    /// Empty means "bearer" (anyone holding the token may connect).
    pub allowed_client_keys: Vec<EndpointId>,
    /// Unique token id, for revocation.
    pub token_id: String,
    /// Not-before, unix ms.
    pub not_before_ms: u64,
    /// Expiry, unix ms.
    pub expires_at_ms: u64,
    /// If true the token may only be redeemed once (pairing code); the host
    /// pins the redeeming client's key on first use.
    pub single_use: bool,
}

/// A signed invite: claims plus a detached signature over their canonical bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedInvite {
    pub claims: InviteClaims,
    /// ed25519 signature over `canonical_bytes(claims)`, hex/base64 on the wire.
    #[serde(with = "sig_bytes")]
    pub signature: Signature,
}

impl InviteClaims {
    /// Canonical, length-prefixed, domain-separated bytes for signing.
    ///
    /// We hand-roll the encoding rather than signing serialized JSON so the
    /// signed message is stable regardless of field ordering or formatting.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        write_field(&mut buf, TOKEN_DOMAIN);
        write_u32(&mut buf, self.version);
        write_field(&mut buf, self.audience.as_bytes());
        write_u32(&mut buf, self.relay_urls.len() as u32);
        for url in &self.relay_urls {
            write_field(&mut buf, url.as_bytes());
        }
        write_u32(&mut buf, scope_tag(self.scope));
        write_u32(&mut buf, self.allowed_client_keys.len() as u32);
        for key in &self.allowed_client_keys {
            write_field(&mut buf, key.as_bytes());
        }
        write_field(&mut buf, self.token_id.as_bytes());
        write_u64(&mut buf, self.not_before_ms);
        write_u64(&mut buf, self.expires_at_ms);
        buf.push(self.single_use as u8);
        buf
    }
}

impl SignedInvite {
    /// Mint a signed invite from the host's secret key.
    pub fn sign(secret: &SecretKey, claims: InviteClaims) -> Self {
        let signature = secret.sign(&claims.canonical_bytes());
        Self { claims, signature }
    }

    /// Verify the signature and validity window at `now_ms`.
    pub fn verify(&self, now_ms: u64) -> Result<(), RoamingError> {
        if self.claims.version != TOKEN_VERSION {
            return Err(RoamingError::Invite(format!(
                "unsupported invite version {}",
                self.claims.version
            )));
        }
        self.claims
            .audience
            .verify(&self.claims.canonical_bytes(), &self.signature)
            .map_err(|_| RoamingError::Invite("bad signature".into()))?;
        if now_ms < self.claims.not_before_ms {
            return Err(RoamingError::Invite("invite not yet valid".into()));
        }
        if now_ms >= self.claims.expires_at_ms {
            return Err(RoamingError::Invite("invite expired".into()));
        }
        Ok(())
    }

    /// Whether `client` is permitted to redeem this invite (ignoring single-use
    /// bookkeeping, which the host tracks separately).
    pub fn permits_client(&self, client: &PublicKey) -> bool {
        self.claims.allowed_client_keys.is_empty()
            || self.claims.allowed_client_keys.contains(client)
    }

    /// The [`EndpointAddr`] a client should dial, reconstructed from the
    /// audience id and relay URLs.
    pub fn endpoint_addr(&self) -> Result<EndpointAddr, RoamingError> {
        let mut addr = EndpointAddr::new(self.claims.audience);
        for url in &self.claims.relay_urls {
            let parsed = url
                .parse()
                .map_err(|_| RoamingError::Invite(format!("bad relay url {url}")))?;
            addr.addrs.insert(TransportAddr::Relay(parsed));
        }
        Ok(addr)
    }

    /// Encode to a compact, URL-safe token string with a `goose+roam://` scheme.
    pub fn encode(&self) -> Result<String, RoamingError> {
        let json = serde_json::to_vec(self)
            .map_err(|e| RoamingError::Invite(format!("encode failed: {e}")))?;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json);
        Ok(format!("goose+roam://{b64}"))
    }

    /// Decode a token string produced by [`SignedInvite::encode`].
    pub fn decode(token: &str) -> Result<Self, RoamingError> {
        let body = token
            .strip_prefix("goose+roam://")
            .ok_or_else(|| RoamingError::Invite("missing goose+roam:// scheme".into()))?;
        let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(body)
            .map_err(|_| RoamingError::Invite("invalid base64".into()))?;
        serde_json::from_slice(&json)
            .map_err(|e| RoamingError::Invite(format!("invalid invite json: {e}")))
    }
}

fn scope_tag(scope: Scope) -> u32 {
    match scope {
        Scope::Control => 1,
        Scope::Attach => 2,
        Scope::Observe => 3,
    }
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_field(buf: &mut Vec<u8>, bytes: &[u8]) {
    write_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

mod sig_bytes {
    use super::Signature;
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sig: &Signature, s: S) -> Result<S::Ok, S::Error> {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());
        s.serialize_str(&b64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Signature, D::Error> {
        let s = String::deserialize(d)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        let arr: [u8; 64] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("signature must be 64 bytes"))?;
        Ok(Signature::from_bytes(&arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(host: &SecretKey) -> InviteClaims {
        InviteClaims {
            version: TOKEN_VERSION,
            audience: host.public(),
            relay_urls: vec!["https://relay.example./".into()],
            scope: Scope::Control,
            allowed_client_keys: vec![],
            token_id: "tok-1".into(),
            not_before_ms: 1_000,
            expires_at_ms: 10_000,
            single_use: false,
        }
    }

    #[test]
    fn signs_and_verifies() {
        let host = SecretKey::generate();
        let invite = SignedInvite::sign(&host, claims(&host));
        assert!(invite.verify(5_000).is_ok());
    }

    #[test]
    fn rejects_expired_and_premature() {
        let host = SecretKey::generate();
        let invite = SignedInvite::sign(&host, claims(&host));
        assert!(invite.verify(500).is_err());
        assert!(invite.verify(20_000).is_err());
    }

    #[test]
    fn rejects_tampered_scope() {
        let host = SecretKey::generate();
        let mut invite = SignedInvite::sign(&host, claims(&host));
        invite.claims.scope = Scope::Observe;
        assert!(invite.verify(5_000).is_err());
    }

    #[test]
    fn token_round_trips() {
        let host = SecretKey::generate();
        let invite = SignedInvite::sign(&host, claims(&host));
        let token = invite.encode().unwrap();
        assert!(token.starts_with("goose+roam://"));
        let decoded = SignedInvite::decode(&token).unwrap();
        assert!(decoded.verify(5_000).is_ok());
        assert_eq!(decoded.claims.token_id, "tok-1");
    }

    #[test]
    fn client_allowlist_enforced() {
        let host = SecretKey::generate();
        let allowed = SecretKey::generate().public();
        let other = SecretKey::generate().public();
        let mut c = claims(&host);
        c.allowed_client_keys = vec![allowed];
        let invite = SignedInvite::sign(&host, c);
        assert!(invite.permits_client(&allowed));
        assert!(!invite.permits_client(&other));
    }

    #[test]
    fn bearer_permits_anyone() {
        let host = SecretKey::generate();
        let invite = SignedInvite::sign(&host, claims(&host));
        assert!(invite.permits_client(&SecretKey::generate().public()));
    }
}
