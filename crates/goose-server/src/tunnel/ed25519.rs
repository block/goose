use anyhow::Result;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

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
        let timestamp = "1731384000";
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
}
