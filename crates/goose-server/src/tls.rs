//! TLS configuration for the goose server.
//!
//! Two TLS backends are supported for the HTTPS listener via `axum-server`:
//!
//! - **`rustls-tls`** (enabled by default) – uses `axum-server/tls-rustls` with
//!   the `aws-lc-rs` crypto provider.
//! - **`native-tls`** – uses `axum-server/tls-openssl`, which links against the
//!   platform's OpenSSL (or a compatible fork such as LibreSSL / BoringSSL).
//!   On Linux this *is* the platform-native TLS stack; on macOS/Windows the
//!   `native-tls` crate used by the HTTP *client* delegates to Security.framework
//!   / SChannel respectively, but `axum-server` does not offer those backends so
//!   the server listener always uses OpenSSL when this feature is active.

use anyhow::Result;
use goose::config::paths::Paths;
use rcgen::{CertificateParams, DnType, KeyPair, SanType};

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
        .push(DnType::CommonName, "goosed localhost");
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

/// Returns the directory used for caching TLS files.
///
/// Uses the same cross-platform config directory as the rest of goose
/// (e.g. `~/.config/goose/tls` on Linux, `~/Library/Application Support/goose/tls`
/// on macOS) so the cached certificate is stored alongside other goose config.
fn get_tls_cache_dir() -> std::path::PathBuf {
    Paths::config_dir().join("tls")
}

/// Write `contents` to `path` with restrictive permissions on Unix (mode 0600).
///
/// On non-Unix platforms the file is written without explicit permission
/// setting.  All errors are silently ignored — callers treat this as
/// best-effort.
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

/// Attempt to load a previously cached TLS configuration from disk.
///
/// Reads `server.pem`, `server.key`, and `server.der` from the cache directory.
/// The SHA-256 fingerprint is **re-derived from the DER bytes** on every load so
/// that a corrupted or tampered pre-computed fingerprint file cannot cause a wrong
/// fingerprint to be announced to the parent process.
///
/// Returns `None` on any error (missing files, parse failures, etc.).
#[cfg(feature = "rustls-tls")]
async fn load_cached_tls() -> Option<TlsSetup> {
    let dir = get_tls_cache_dir();
    let cert_bytes = std::fs::read(dir.join("server.pem")).ok()?;
    let key_bytes = std::fs::read(dir.join("server.key")).ok()?;
    let cert_der = std::fs::read(dir.join("server.der")).ok()?;

    // Derive fingerprint from the actual cert bytes rather than trusting a
    // stored string.
    let fingerprint = sha256_fingerprint(&cert_der);

    let config = axum_server::tls_rustls::RustlsConfig::from_pem(cert_bytes, key_bytes)
        .await
        .ok()?;

    Some(TlsSetup {
        config,
        fingerprint,
    })
}

#[cfg(feature = "native-tls")]
async fn load_cached_tls() -> Option<TlsSetup> {
    let dir = get_tls_cache_dir();
    let cert_bytes = std::fs::read(dir.join("server.pem")).ok()?;
    let key_bytes = std::fs::read(dir.join("server.key")).ok()?;
    let cert_der = std::fs::read(dir.join("server.der")).ok()?;

    // Derive fingerprint from the actual cert bytes rather than trusting a
    // stored string.
    let fingerprint = sha256_fingerprint(&cert_der);

    let config = axum_server::tls_openssl::OpenSSLConfig::from_pem(&cert_bytes, &key_bytes).ok()?;

    Some(TlsSetup {
        config,
        fingerprint,
    })
}

/// Persist the TLS certificate, private key, and raw DER bytes to the cache
/// directory so they can be reused on the next startup.
///
/// The DER bytes (not a pre-computed fingerprint string) are stored so that
/// the fingerprint is always derived from the actual certificate on load.
/// The private key is written with mode 0600 on Unix so it is only readable
/// by the current user.
///
/// All errors are silently ignored — this is a best-effort optimisation and
/// must never prevent the server from starting.
fn save_tls_to_cache(cert_pem: &str, cert_der: &[u8], key_pem: &str) {
    let dir = get_tls_cache_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join("server.pem"), cert_pem);
    let _ = std::fs::write(dir.join("server.der"), cert_der);
    write_private_key(&dir.join("server.key"), key_pem.as_bytes());
}

/// Generate a self-signed TLS certificate for localhost (127.0.0.1) and
/// return a [`TlsSetup`] containing the server config and the SHA-256
/// fingerprint of the generated certificate (colon-separated hex).
///
/// The fingerprint is printed to stdout so the parent process (e.g. Electron)
/// can pin it and reject connections from any other certificate.
pub async fn self_signed_config() -> Result<TlsSetup> {
    #[cfg(feature = "rustls-tls")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Fast path: reuse a previously cached certificate if one exists.
    if let Some(cached) = load_cached_tls().await {
        println!("GOOSED_CERT_FINGERPRINT={}", cached.fingerprint);
        return Ok(cached);
    }

    let (cert, key_pair) = generate_self_signed_cert()?;

    let fingerprint = sha256_fingerprint(cert.der());
    println!("GOOSED_CERT_FINGERPRINT={fingerprint}");

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    // Persist for future restarts before moving the strings into the config.
    save_tls_to_cache(&cert_pem, cert.der(), &key_pem);

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
