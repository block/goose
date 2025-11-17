use anyhow::Result;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Ed25519Validator {
    public_key: VerifyingKey,
}

impl Ed25519Validator {
    pub fn new(public_key_hex: &str) -> Result<Self> {
        let bytes = hex::decode(public_key_hex)?;
        if bytes.len() != 32 {
            anyhow::bail!("Public key must be 32 bytes");
        }
        let public_key =
            VerifyingKey::from_bytes(&bytes.try_into().expect("verified 32 bytes above"))?;
        Ok(Self { public_key })
    }

    pub fn verify(
        &self,
        signature_header: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<()> {
        let parts: Vec<&str> = signature_header.split('.').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid signature format");
        }

        let timestamp = parts[0];
        let sig_hex = parts[1];

        let timestamp_secs: i64 = timestamp
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid timestamp format"))?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let age = now - timestamp_secs;

        const MAX_AGE_SECS: i64 = 300;
        if age > MAX_AGE_SECS {
            anyhow::bail!("Signature expired (age: {}s)", age);
        }
        if age < -MAX_AGE_SECS {
            anyhow::bail!("Signature timestamp too far in future");
        }

        let body_hash = if let Some(body_content) = body {
            let hash = Sha256::digest(body_content.as_bytes());
            hex::encode(hash)
        } else {
            String::new()
        };

        let message = format!("{}|{}|{}|{}", method, path, timestamp, body_hash);
        let sig_bytes = hex::decode(sig_hex)?;
        let signature = Signature::from_bytes(
            &sig_bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?,
        );

        self.public_key.verify(message.as_bytes(), &signature)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    #[allow(clippy::zero_prefixed_literal)]
    fn test_signature_validation() {
        let signing_key = SigningKey::from_bytes(&[
            157, 097, 177, 157, 239, 253, 090, 096, 186, 132, 074, 244, 146, 236, 044, 196, 068,
            073, 197, 105, 123, 050, 105, 025, 112, 059, 172, 003, 028, 174, 127, 096,
        ]);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.as_bytes());

        let validator = Ed25519Validator::new(&public_key_hex).unwrap();

        let method = "POST";
        let path = "/api/test";
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let body = r#"{"test":"data"}"#;

        let body_hash = hex::encode(Sha256::digest(body.as_bytes()));
        let message = format!("{}|{}|{}|{}", method, path, timestamp, body_hash);
        let signature = signing_key.sign(message.as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());

        let signature_header = format!("{}.{}", timestamp, sig_hex);

        assert!(validator
            .verify(&signature_header, method, path, Some(body))
            .is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.as_bytes());

        let validator = Ed25519Validator::new(&public_key_hex).unwrap();

        let signature_header = "1731384000.deadbeef";
        assert!(validator
            .verify(signature_header, "GET", "/test", None)
            .is_err());
    }

    #[test]
    fn test_expired_signature() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.as_bytes());

        let validator = Ed25519Validator::new(&public_key_hex).unwrap();

        let method = "GET";
        let path = "/test";
        let old_timestamp = "1731384000";

        let message = format!("{}|{}|{}|", method, path, old_timestamp);
        let signature = signing_key.sign(message.as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());
        let signature_header = format!("{}.{}", old_timestamp, sig_hex);

        let result = validator.verify(&signature_header, method, path, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signature expired"));
    }

    #[test]
    fn test_future_timestamp() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.as_bytes());

        let validator = Ed25519Validator::new(&public_key_hex).unwrap();

        let method = "GET";
        let path = "/test";
        let future_timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 400)
            .to_string();

        let message = format!("{}|{}|{}|", method, path, future_timestamp);
        let signature = signing_key.sign(message.as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());
        let signature_header = format!("{}.{}", future_timestamp, sig_hex);

        let result = validator.verify(&signature_header, method, path, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("too far in future"));
    }
}
