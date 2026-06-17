use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use image::GenericImageView;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::edit::resolve_path;

const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// Maximum number of redirects followed when fetching a remote image. Each hop
/// is re-validated against the SSRF address rules before it is dialed.
const MAX_IMAGE_REDIRECTS: usize = 5;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImageReadParams {
    /// Local file path or http(s) URL of the image to load.
    pub source: String,
    /// Optional crop rectangle in pixels. Coordinates are measured from the top-left corner.
    /// use to zoom in and get more details.
    #[serde(default)]
    pub crop: Option<CropParams>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CropParams {
    /// Left edge of the crop rectangle in pixels.
    pub x: u32,
    /// Top edge of the crop rectangle in pixels.
    pub y: u32,
    /// Width of the crop rectangle in pixels.
    pub width: u32,
    /// Height of the crop rectangle in pixels.
    pub height: u32,
}

pub struct ImageTool;

impl ImageTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn image_read_with_cwd(
        &self,
        params: ImageReadParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        match load_image(&params, working_dir).await {
            Ok(loaded) => {
                let mut result = CallToolResult::success(vec![
                    Content::text(loaded.summary(&params.source)).with_priority(0.0),
                    Content::image(loaded.data, loaded.mime_type.clone()).with_priority(0.0),
                ]);
                result.structured_content = Some(json!({
                    "source": params.source,
                    "mimeType": loaded.mime_type,
                    "width": loaded.width,
                    "height": loaded.height,
                    "bytes": loaded.bytes_len,
                    "originalWidth": loaded.original_width,
                    "originalHeight": loaded.original_height,
                    "crop": params.crop,
                }));
                result
            }
            Err(error) => CallToolResult::error(vec![
                Content::text(format!("Error: {error}")).with_priority(0.0)
            ]),
        }
    }
}

impl Default for ImageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct LoadedImage {
    data: String,
    mime_type: String,
    bytes_len: usize,
    width: u32,
    height: u32,
    original_width: u32,
    original_height: u32,
    cropped: bool,
}

impl LoadedImage {
    fn summary(&self, source: &str) -> String {
        let crop_note = if self.cropped {
            format!(
                " Cropped from {}x{} to {}x{}.",
                self.original_width, self.original_height, self.width, self.height
            )
        } else {
            String::new()
        };

        format!(
            "Loaded image from {source} ({} bytes, {}, {}x{}).{crop_note}",
            self.bytes_len, self.mime_type, self.width, self.height
        )
    }
}

async fn load_image(
    params: &ImageReadParams,
    working_dir: Option<&Path>,
) -> Result<LoadedImage, String> {
    if params.source.trim().is_empty() {
        return Err("source cannot be empty".to_string());
    }

    let bytes = load_image_bytes(&params.source, working_dir).await?;
    ensure_image_size(bytes.len() as u64)?;

    let format = image::guess_format(&bytes).map_err(|_| {
        "unsupported image format; supported formats are png, jpeg, gif, and webp".to_string()
    })?;
    let mime_type = mime_type(format)?;
    let image = image::load_from_memory_with_format(&bytes, format)
        .map_err(|error| format!("failed to decode image: {error}"))?;
    let (original_width, original_height) = image.dimensions();

    let Some(crop) = &params.crop else {
        return Ok(LoadedImage {
            data: base64::prelude::BASE64_STANDARD.encode(&bytes),
            mime_type: mime_type.to_string(),
            bytes_len: bytes.len(),
            width: original_width,
            height: original_height,
            original_width,
            original_height,
            cropped: false,
        });
    };

    validate_crop(crop, original_width, original_height)?;
    let cropped = image.crop_imm(crop.x, crop.y, crop.width, crop.height);
    let mut cropped_bytes = Cursor::new(Vec::new());
    cropped
        .write_to(&mut cropped_bytes, image::ImageFormat::Png)
        .map_err(|error| format!("failed to encode cropped image: {error}"))?;
    let cropped_bytes = cropped_bytes.into_inner();
    ensure_image_size(cropped_bytes.len() as u64)?;

    Ok(LoadedImage {
        data: base64::prelude::BASE64_STANDARD.encode(&cropped_bytes),
        mime_type: "image/png".to_string(),
        bytes_len: cropped_bytes.len(),
        width: crop.width,
        height: crop.height,
        original_width,
        original_height,
        cropped: true,
    })
}

async fn load_image_bytes(source: &str, working_dir: Option<&Path>) -> Result<Vec<u8>, String> {
    if let Ok(url) = url::Url::parse(source) {
        match url.scheme() {
            "http" | "https" => load_url_bytes(url).await,
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|_| "invalid file URL".to_string())?;
                load_file_bytes(path)
            }
            _ => load_file_bytes(resolve_path(source, working_dir)),
        }
    } else {
        load_file_bytes(resolve_path(source, working_dir))
    }
}

async fn load_url_bytes(url: url::Url) -> Result<Vec<u8>, String> {
    // SSRF guard. Reject a literal-IP host up front for a clear error, then let
    // the request proceed through a hardened client where the *dialed* address
    // is validated on every DNS resolution (defeating DNS rebinding) and every
    // redirect hop is validated against the same rules before it is followed.
    if let Some(host) = url.host_str() {
        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_disallowed_ip(ip) {
                return Err(blocked_target_message(host));
            }
        }
    } else {
        return Err("image URL is missing a host".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .dns_resolver(Arc::new(GuardedResolver))
        .redirect(guarded_redirect_policy())
        .build()
        .map_err(|error| format!("failed to create HTTP client: {error}"))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("failed to download image: {error}"))?
        .error_for_status()
        .map_err(|error| format!("failed to download image: {error}"))?;

    if let Some(len) = response.content_length() {
        ensure_image_size(len)?;
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("failed to read image response: {error}"))?;

    Ok(bytes.to_vec())
}

/// Box a message into the boxed-error type reqwest's resolver contract expects.
fn box_err(message: String) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::other(message).into()
}

/// reqwest DNS resolver that validates every resolved address against the SSRF
/// rules. Because reqwest calls this at connection time, the address it actually
/// dials is guaranteed to have passed validation — a host that returns a public
/// address to a preflight check and a private one to the real request (DNS
/// rebinding) is still rejected here, on the resolution that gets dialed.
struct GuardedResolver;

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            // Port 0: reqwest substitutes the scheme/URL port onto the returned
            // addresses, so the lookup port is irrelevant to the dialed target.
            let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|error| box_err(format!("failed to resolve image host {host}: {error}")))?
                .collect();

            let allowed = filter_resolved_addrs(&host, resolved).map_err(box_err)?;
            let addrs: Addrs = Box::new(allowed.into_iter());
            Ok(addrs)
        })
    }
}

/// Keep only publicly routable addresses from a host resolution.
///
/// Errors if the host resolved to nothing, or if *every* resolved address is
/// non-public (the rebinding case: a private-only answer at dial time must be
/// rejected, not silently dialed).
fn filter_resolved_addrs(host: &str, resolved: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, String> {
    if resolved.is_empty() {
        return Err(format!("failed to resolve image host {host}"));
    }

    let allowed: Vec<SocketAddr> = resolved
        .into_iter()
        .filter(|addr| !is_disallowed_ip(addr.ip()))
        .collect();

    if allowed.is_empty() {
        return Err(blocked_target_message(host));
    }

    Ok(allowed)
}

/// Redirect policy that follows redirects only toward public destinations.
///
/// Hostname targets are revalidated by [`GuardedResolver`] when the next hop is
/// dialed, so this policy focuses on what the resolver cannot see: it rejects a
/// literal-IP `Location` that points at a non-public address (a redirect that
/// would otherwise skip DNS), rejects non-`http(s)` schemes, and caps the number
/// of hops. Legitimate public redirects (HTTP→HTTPS upgrades, CDN links) are
/// still followed.
fn guarded_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        match classify_redirect_target(attempt.url(), attempt.previous().len()) {
            RedirectDecision::Follow => attempt.follow(),
            RedirectDecision::Reject(message) => attempt.error(message),
        }
    })
}

enum RedirectDecision {
    Follow,
    Reject(String),
}

/// Decide whether a redirect hop may be followed, using the same address rules
/// as the resolver. Hostname targets are allowed here and revalidated by
/// [`GuardedResolver`] at dial time; literal-IP targets are checked directly so
/// a redirect cannot reach a non-public address by bypassing DNS.
fn classify_redirect_target(next: &url::Url, hops_so_far: usize) -> RedirectDecision {
    if hops_so_far >= MAX_IMAGE_REDIRECTS {
        return RedirectDecision::Reject(format!(
            "too many redirects while fetching image (max {MAX_IMAGE_REDIRECTS})"
        ));
    }

    match next.scheme() {
        "http" | "https" => {}
        other => {
            return RedirectDecision::Reject(format!(
                "refusing to follow redirect to {other} scheme"
            ));
        }
    }

    match next.host_str() {
        None => RedirectDecision::Reject("redirect target is missing a host".to_string()),
        Some(host) => match host.parse::<IpAddr>() {
            Ok(ip) if is_disallowed_ip(ip) => {
                RedirectDecision::Reject(blocked_target_message(host))
            }
            _ => RedirectDecision::Follow,
        },
    }
}

fn load_file_bytes(path: PathBuf) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("failed to read image file: {error}"))
}

fn blocked_target_message(host: &str) -> String {
    format!(
        "refusing to fetch image from non-public address for host {host}: \
         loopback, private, link-local, and cloud-metadata targets are blocked"
    )
}

/// Returns true if the address must not be fetched from (non-public / internal).
fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_disallowed_ipv4(v4),
        IpAddr::V6(v6) => {
            // Handle IPv4-mapped / IPv4-compatible addresses by checking the
            // embedded IPv4 address with the v4 rules as well.
            if let Some(v4) = v6.to_ipv4() {
                if is_disallowed_ipv4(v4) {
                    return true;
                }
            }
            is_disallowed_ipv6(v6)
        }
    }
}

fn is_disallowed_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        // 100.64.0.0/10 carrier-grade NAT (shared address space, RFC 6598).
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 0x40)
        // 192.0.0.0/24 IETF protocol assignments.
        || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
        // 198.18.0.0/15 benchmarking.
        || (ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18)
}

fn is_disallowed_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    let segments = ip.segments();
    // fe80::/10 link-local (also covers the IPv6 metadata route fe80::a9fe:...).
    let link_local = (segments[0] & 0xffc0) == 0xfe80;
    // fc00::/7 unique-local addresses.
    let unique_local = (segments[0] & 0xfe00) == 0xfc00;
    // ::ffff:0:0/96 IPv4-mapped handled separately, but reject ::/96-style
    // IPv4-compatible documentation ranges defensively via 2001:db8::/32.
    let documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    link_local || unique_local || documentation
}

fn validate_crop(crop: &CropParams, image_width: u32, image_height: u32) -> Result<(), String> {
    if crop.width == 0 || crop.height == 0 {
        return Err("crop width and height must be greater than zero".to_string());
    }

    let right = crop
        .x
        .checked_add(crop.width)
        .ok_or_else(|| "crop rectangle is out of bounds".to_string())?;
    let bottom = crop
        .y
        .checked_add(crop.height)
        .ok_or_else(|| "crop rectangle is out of bounds".to_string())?;

    if right > image_width || bottom > image_height {
        return Err(format!(
            "crop rectangle {}x{} at {},{} exceeds image bounds {}x{}",
            crop.width, crop.height, crop.x, crop.y, image_width, image_height
        ));
    }

    Ok(())
}

fn ensure_image_size(len: u64) -> Result<(), String> {
    if len > MAX_IMAGE_BYTES {
        Err(format!(
            "image is too large: {len} bytes exceeds {MAX_IMAGE_BYTES} byte limit"
        ))
    } else {
        Ok(())
    }
}

fn mime_type(format: image::ImageFormat) -> Result<&'static str, String> {
    match format {
        image::ImageFormat::Png => Ok("image/png"),
        image::ImageFormat::Jpeg => Ok("image/jpeg"),
        image::ImageFormat::Gif => Ok("image/gif"),
        image::ImageFormat::WebP => Ok("image/webp"),
        _ => Err(
            "unsupported image format; supported formats are png, jpeg, gif, and webp".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(source: &str) -> ImageReadParams {
        ImageReadParams {
            source: source.to_string(),
            crop: None,
        }
    }

    #[test]
    fn disallows_loopback_private_and_metadata_ips() {
        // Loopback (v4/v6).
        assert!(is_disallowed_ip("127.0.0.1".parse().unwrap()));
        assert!(is_disallowed_ip("::1".parse().unwrap()));
        // RFC1918 private ranges.
        assert!(is_disallowed_ip("10.0.0.1".parse().unwrap()));
        assert!(is_disallowed_ip("172.16.5.4".parse().unwrap()));
        assert!(is_disallowed_ip("192.168.1.1".parse().unwrap()));
        // Cloud metadata endpoint / link-local.
        assert!(is_disallowed_ip("169.254.169.254".parse().unwrap()));
        // Carrier-grade NAT and benchmarking ranges.
        assert!(is_disallowed_ip("100.64.1.1".parse().unwrap()));
        assert!(is_disallowed_ip("198.18.0.1".parse().unwrap()));
        // Unique-local and link-local IPv6.
        assert!(is_disallowed_ip("fc00::1".parse().unwrap()));
        assert!(is_disallowed_ip("fe80::1".parse().unwrap()));
        // IPv4-mapped IPv6 form of loopback must also be blocked.
        assert!(is_disallowed_ip("::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn allows_public_ips() {
        assert!(!is_disallowed_ip("1.1.1.1".parse().unwrap()));
        assert!(!is_disallowed_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_disallowed_ip("93.184.216.34".parse().unwrap()));
        assert!(!is_disallowed_ip("2606:4700:4700::1111".parse().unwrap()));
    }

    fn addr(s: &str) -> SocketAddr {
        format!("{s}:80").parse().unwrap()
    }

    #[tokio::test]
    async fn load_url_bytes_rejects_literal_loopback() {
        let url = url::Url::parse("http://127.0.0.1:8080/probe.png").unwrap();
        let err = load_url_bytes(url)
            .await
            .expect_err("loopback target must be rejected before any fetch");
        assert!(
            err.contains("non-public address"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn load_url_bytes_rejects_metadata_endpoint() {
        let url = url::Url::parse("http://169.254.169.254/latest/meta-data/").unwrap();
        let err = load_url_bytes(url)
            .await
            .expect_err("cloud metadata endpoint must be rejected");
        assert!(
            err.contains("non-public address"),
            "unexpected error: {err}"
        );
    }

    // --- P1: the dial-time resolver filter (DNS rebinding) ---------------------

    #[test]
    fn resolver_filter_rejects_when_all_resolved_addrs_are_private() {
        // A rebinding host that answers only with a loopback/private address at
        // dial time must be rejected, not dialed.
        let err =
            filter_resolved_addrs("rebind.example", vec![addr("127.0.0.1"), addr("10.0.0.5")])
                .expect_err("private-only resolution must be rejected");
        assert!(
            err.contains("non-public address"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn resolver_filter_drops_private_keeps_public() {
        // Mixed answer: only the public address survives, so a private address
        // mixed into the answer can never be the one reqwest dials.
        let allowed = filter_resolved_addrs(
            "mixed.example",
            vec![addr("169.254.169.254"), addr("93.184.216.34")],
        )
        .expect("a public address remains");
        assert_eq!(allowed, vec![addr("93.184.216.34")]);
    }

    #[test]
    fn resolver_filter_errors_on_empty_resolution() {
        let err =
            filter_resolved_addrs("nx.example", vec![]).expect_err("empty resolution is an error");
        assert!(err.contains("failed to resolve"), "unexpected error: {err}");
    }

    // --- P2: the redirect policy --------------------------------------------------

    #[test]
    fn redirect_to_private_literal_ip_is_rejected() {
        let target = url::Url::parse("http://127.0.0.1/internal").unwrap();
        match classify_redirect_target(&target, 0) {
            RedirectDecision::Reject(msg) => {
                assert!(msg.contains("non-public address"), "unexpected: {msg}")
            }
            RedirectDecision::Follow => panic!("redirect to loopback must be rejected"),
        }
    }

    #[test]
    fn redirect_to_metadata_literal_ip_is_rejected() {
        let target = url::Url::parse("http://169.254.169.254/latest/meta-data/").unwrap();
        assert!(matches!(
            classify_redirect_target(&target, 0),
            RedirectDecision::Reject(_)
        ));
    }

    #[test]
    fn redirect_to_public_target_is_followed() {
        // Public literal IP and a hostname (revalidated by the resolver at dial
        // time) are both allowed to proceed.
        let public_ip = url::Url::parse("https://93.184.216.34/image.png").unwrap();
        assert!(matches!(
            classify_redirect_target(&public_ip, 0),
            RedirectDecision::Follow
        ));
        let hostname = url::Url::parse("https://cdn.example.com/image.png").unwrap();
        assert!(matches!(
            classify_redirect_target(&hostname, 1),
            RedirectDecision::Follow
        ));
    }

    #[test]
    fn redirect_to_non_http_scheme_is_rejected() {
        let target = url::Url::parse("file:///etc/passwd").unwrap();
        assert!(matches!(
            classify_redirect_target(&target, 0),
            RedirectDecision::Reject(_)
        ));
    }

    #[test]
    fn redirect_hop_cap_is_enforced() {
        let target = url::Url::parse("https://cdn.example.com/image.png").unwrap();
        match classify_redirect_target(&target, MAX_IMAGE_REDIRECTS) {
            RedirectDecision::Reject(msg) => {
                assert!(msg.contains("too many redirects"), "unexpected: {msg}")
            }
            RedirectDecision::Follow => panic!("hop cap must stop the redirect chain"),
        }
    }

    #[tokio::test]
    async fn load_image_rejects_loopback_url_without_fetching() {
        // Port 1 is unbound; on the unpatched code this would attempt a TCP
        // connection and fail with a generic download error. With the SSRF
        // guard in place the request is refused up-front with a clear message.
        let result = load_image(&params("http://127.0.0.1:1/probe.png"), None).await;
        let err = result.expect_err("loopback image URL must be rejected");
        assert!(
            err.contains("non-public address"),
            "expected SSRF guard rejection, got: {err}"
        );
    }

    #[tokio::test]
    async fn load_image_allows_local_file_paths() {
        // A non-URL source still resolves as a filesystem path; a missing file
        // yields a read error (not an SSRF rejection), proving the guard does
        // not interfere with the local-file code path.
        let result = load_image(&params("definitely-not-a-real-image.png"), None).await;
        let err = result.expect_err("missing file should error");
        assert!(
            err.contains("failed to read image file"),
            "unexpected error for local path: {err}"
        );
    }
}
