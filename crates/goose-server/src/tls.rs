use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use ring::digest;

pub struct TlsSetup {
    pub config: RustlsConfig,
    pub fingerprint: String,
}

/// Generate a self-signed TLS certificate for localhost (127.0.0.1) and
/// return a [`TlsSetup`] containing the rustls config and the SHA-256
/// fingerprint of the generated certificate (colon-separated hex).
///
/// The fingerprint is printed to stdout so the parent process (e.g. Electron)
/// can pin it and reject connections from any other certificate.
pub async fn self_signed_config() -> Result<TlsSetup> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "goosed localhost");
    params.subject_alt_names = vec![
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        SanType::DnsName("localhost".try_into()?),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = cert.der();
    let sha256 = digest::digest(&digest::SHA256, cert_der);
    let fingerprint = sha256
        .as_ref()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");

    println!("GOOSED_CERT_FINGERPRINT={fingerprint}");

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    let config = RustlsConfig::from_pem(cert_pem.into_bytes(), key_pem.into_bytes()).await?;

    Ok(TlsSetup {
        config,
        fingerprint,
    })
}
