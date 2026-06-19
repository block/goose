use crate::config::Config;
use crate::providers::api_client::TlsConfig;
use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn provider_tls_config_from_config(config: &Config) -> Result<Option<TlsConfig>> {
    let mut tls_config = TlsConfig::new();
    let mut has_tls_config = false;

    let client_cert_path = config.get_param::<String>("GOOSE_CLIENT_CERT_PATH").ok();
    let client_key_path = config.get_param::<String>("GOOSE_CLIENT_KEY_PATH").ok();

    match (client_cert_path, client_key_path) {
        (Some(cert_path), Some(key_path)) => {
            tls_config = tls_config
                .with_client_cert_and_key(PathBuf::from(cert_path), PathBuf::from(key_path));
            has_tls_config = true;
        }
        (Some(_), None) => {
            bail!(
                "Client certificate provided (GOOSE_CLIENT_CERT_PATH) but no private key (GOOSE_CLIENT_KEY_PATH)"
            );
        }
        (None, Some(_)) => {
            bail!(
                "Client private key provided (GOOSE_CLIENT_KEY_PATH) but no certificate (GOOSE_CLIENT_CERT_PATH)"
            );
        }
        (None, None) => {}
    }

    if let Ok(ca_cert_path) = config.get_param::<String>("GOOSE_CA_CERT_PATH") {
        tls_config = tls_config.with_ca_cert(PathBuf::from(ca_cert_path));
        has_tls_config = true;
    }

    Ok(has_tls_config.then_some(tls_config))
}
