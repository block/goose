use crate::config::paths::Paths;
use anyhow::{bail, Result};
use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use std::path::Path;

#[cfg(feature = "rustls-tls")]
pub type TlsConfig = axum_server::tls_rustls::RustlsConfig;

#[cfg(feature = "native-tls")]
pub type TlsConfig = axum_server::tls_openssl::OpenSSLConfig;

pub struct TlsSetup {
    pub config: TlsConfig,
    pub fingerprint: String,
}

fn generate_self_signed_cert() -> Result<(rcgen::Certificate, KeyPair)> {
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "goslingd localhost");
    params.subject_alt_names = vec![
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        SanType::DnsName("localhost".try_into()?),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    Ok((cert, key_pair))
}

fn sha256_fingerprint(der: &[u8]) -> String {
    #[cfg(feature = "rustls-tls")]
    {
        let sha256 = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, der);
        sha256
            .as_ref()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    #[cfg(feature = "native-tls")]
    {
        use openssl::hash::MessageDigest;
        let digest =
            openssl::hash::hash(MessageDigest::sha256(), der).expect("SHA-256 hash failed");
        digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

pub async fn from_pem_files(cert_path: &Path, key_path: &Path) -> Result<TlsSetup> {
    let cert_pem = std::fs::read(cert_path)?;
    let key_pem = std::fs::read(key_path)?;

    let der = pem::parse(&cert_pem)?.into_contents();
    let fingerprint = sha256_fingerprint(&der);
    println!("GOSLINGD_CERT_FINGERPRINT={fingerprint}");

    #[cfg(feature = "rustls-tls")]
    let config = {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem.clone()).await?
    };

    #[cfg(feature = "native-tls")]
    let config = axum_server::tls_openssl::OpenSSLConfig::from_pem(&cert_pem, &key_pem)?;

    Ok(TlsSetup {
        config,
        fingerprint,
    })
}

pub async fn setup_tls(cert_path: Option<&str>, key_path: Option<&str>) -> Result<TlsSetup> {
    match (cert_path, key_path) {
        (Some(cert), Some(key)) => from_pem_files(Path::new(cert), Path::new(key)).await,
        (None, None) => self_signed_config().await,
        _ => bail!("Both GOSLING_TLS_CERT_PATH and GOSLING_TLS_KEY_PATH must be set, or neither"),
    }
}

fn tls_cache_dir() -> std::path::PathBuf {
    Paths::config_dir().join("tls")
}

fn write_private_key(path: &std::path::Path, contents: &[u8]) {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let result = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path);
        if let Ok(mut file) = result {
            let _ = file.write_all(contents);
        }
    }

    #[cfg(not(unix))]
    {
        let _ = std::fs::write(path, contents);
    }
}

async fn load_cached_tls() -> Option<TlsSetup> {
    let dir = tls_cache_dir();
    let cert_pem = std::fs::read(dir.join("server.pem")).ok()?;
    let key_pem = std::fs::read(dir.join("server.key")).ok()?;

    let der = pem::parse(&cert_pem).ok()?.into_contents();
    let fingerprint = sha256_fingerprint(&der);

    #[cfg(feature = "rustls-tls")]
    let config = axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem)
        .await
        .ok()?;
    #[cfg(feature = "native-tls")]
    let config = axum_server::tls_openssl::OpenSSLConfig::from_pem(&cert_pem, &key_pem).ok()?;

    Some(TlsSetup {
        config,
        fingerprint,
    })
}

fn try_save_tls_to_cache(cert_pem: &str, key_pem: &str) {
    let dir = tls_cache_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join("server.pem"), cert_pem);
    write_private_key(&dir.join("server.key"), key_pem.as_bytes());
}

pub async fn self_signed_config() -> Result<TlsSetup> {
    #[cfg(feature = "rustls-tls")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    if let Some(cached) = load_cached_tls().await {
        println!("GOSLINGD_CERT_FINGERPRINT={}", cached.fingerprint);
        return Ok(cached);
    }

    let (cert, key_pair) = generate_self_signed_cert()?;

    let fingerprint = sha256_fingerprint(cert.der());
    println!("GOSLINGD_CERT_FINGERPRINT={fingerprint}");

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    try_save_tls_to_cache(&cert_pem, &key_pem);

    #[cfg(feature = "rustls-tls")]
    let config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert_pem.into_bytes(),
        key_pem.into_bytes(),
    )
    .await?;

    #[cfg(feature = "native-tls")]
    let config =
        axum_server::tls_openssl::OpenSSLConfig::from_pem(cert_pem.as_bytes(), key_pem.as_bytes())?;

    Ok(TlsSetup {
        config,
        fingerprint,
    })
}
