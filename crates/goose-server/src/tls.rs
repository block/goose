use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertificateParams, DnType, KeyPair, SanType};

/// Generate a self-signed TLS certificate for localhost (127.0.0.1) and
/// return an `axum_server::tls_rustls::RustlsConfig` ready to use.
pub async fn self_signed_config() -> Result<RustlsConfig> {
    // rustls 0.23+ requires an explicit crypto provider installation.
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

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    let config = RustlsConfig::from_pem(cert_pem.into_bytes(), key_pem.into_bytes()).await?;

    Ok(config)
}
