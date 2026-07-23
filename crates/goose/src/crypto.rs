#[cfg(feature = "rustls-tls")]
fn default_tls_provider() -> rustls::crypto::CryptoProvider {
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    provider
        .kx_groups
        .retain(|group| group.name() != rustls::NamedGroup::X25519MLKEM768);
    provider
}

/// Installs Goose's process-wide Rustls provider before any TLS configuration is built.
#[cfg(feature = "rustls-tls")]
pub fn install_default_tls_provider() {
    let _ = default_tls_provider().install_default();
}

/// No-op when Goose is built with the native TLS backend.
#[cfg(not(feature = "rustls-tls"))]
pub fn install_default_tls_provider() {}

#[cfg(all(test, feature = "rustls-tls"))]
mod tests {
    use super::default_tls_provider;

    #[test]
    fn default_provider_excludes_mlkem_hybrid_group() {
        let provider = default_tls_provider();

        assert!(provider
            .kx_groups
            .iter()
            .all(|group| group.name() != rustls::NamedGroup::X25519MLKEM768));
        assert!(provider
            .kx_groups
            .iter()
            .any(|group| group.name() == rustls::NamedGroup::X25519));
    }
}
