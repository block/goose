use crate::config::signup_aimlapi::{
    PkceAuthFlow, AIMLAPI_APP_URL_DEFAULT, AIMLAPI_PARTNER_ID_DEFAULT, AIMLAPI_WEB_URL_DEFAULT,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

#[test]
fn challenge_is_the_s256_hash_of_the_verifier() {
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    let mut hasher = Sha256::new();
    hasher.update(flow.code_verifier.as_bytes());
    let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());

    assert_eq!(flow.code_challenge, expected);
}

#[test]
fn challenge_and_state_are_url_safe_and_unpadded() {
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    for value in [&flow.code_challenge, &flow.state] {
        assert!(!value.contains('='), "{value} is padded");
        assert!(!value.contains('+'), "{value} is not url-safe");
        assert!(!value.contains('/'), "{value} is not url-safe");
    }
}

#[test]
fn each_flow_gets_its_own_verifier_state_and_challenge() {
    let a = PkceAuthFlow::new().expect("Failed to create PKCE flow 1");
    let b = PkceAuthFlow::new().expect("Failed to create PKCE flow 2");

    assert_ne!(a.code_verifier, b.code_verifier);
    assert_ne!(a.code_challenge, b.code_challenge);
    assert_ne!(a.state, b.state);
}

#[test]
fn consent_base_keeps_the_app_path() {
    // The server appends "/agent/authorize" to whatever base it is handed. The
    // web app is served under an "/app/" base path, so dropping it produces
    // https://aimlapi.com/agent/authorize — a 404 that strands the user on the
    // first step of the flow. This has to survive future tidying of the URL.
    assert!(
        AIMLAPI_WEB_URL_DEFAULT.ends_with("/app"),
        "consent base must carry the /app path, got {AIMLAPI_WEB_URL_DEFAULT}"
    );
}

#[test]
fn api_and_consent_hosts_are_distinct() {
    // The registration/exchange calls go to the API host; only the browser is
    // sent to the consent host. Collapsing the two would send API calls to the
    // marketing site.
    assert_ne!(AIMLAPI_APP_URL_DEFAULT, AIMLAPI_WEB_URL_DEFAULT);
    assert!(AIMLAPI_APP_URL_DEFAULT.starts_with("https://"));
    assert!(AIMLAPI_WEB_URL_DEFAULT.starts_with("https://"));
}

#[test]
fn partner_id_matches_the_gateway_pattern() {
    // The gateway only attributes ids shaped part_<alnum>; anything else is
    // treated as untagged usage and earns nothing.
    let id = AIMLAPI_PARTNER_ID_DEFAULT;

    assert!(id.starts_with("part_"), "{id} lacks the part_ prefix");
    let rest = &id["part_".len()..];
    assert!(!rest.is_empty(), "{id} has an empty body");
    assert!(
        rest.chars().all(|c| c.is_ascii_alphanumeric()),
        "{id} has non-alphanumeric characters after the prefix"
    );
}
