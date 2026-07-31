//! Browser (wasm) roaming client.
//!
//! Runs iroh **in the browser**, relay-only (QUIC-over-WebSocket-to-relay), to
//! connect to a `goose roam share` host and expose the authorized ACP byte
//! stream to JS. It deliberately knows nothing about ACP itself: after the roam
//! handshake it hands JS a raw byte duplex (`send` / `recv`), which the JS side
//! feeds to `@agentclientprotocol/sdk`'s `ndJsonStream` + `ClientSideConnection`.
//!
//! No Tauri, no native process — the browser tab is the roam peer.

use futures::io::{AsyncReadExt, AsyncWriteExt};
use iroh::{
    endpoint::{presets::Minimal, Connection},
    Endpoint, EndpointAddr, EndpointId, RelayConfig, RelayMap, RelayMode, SecretKey, TransportAddr,
};
use serde::{Deserialize, Serialize};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

const ROAMING_ACP_ALPN: &[u8] = b"goose-acp/1";
const CARD_SCHEME: &str = "goose+roam://";
const MAX_FRAME_BYTES: u32 = 64 * 1024;
/// Bounded so a slow ACP reader/writer applies backpressure rather than growing
/// an unbounded browser-side queue.
const CHANNEL_CAP: usize = 16;
const READ_CHUNK: usize = 16 * 1024;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

/// Mirrors `goose-roaming`'s `ConnectionCard` wire form (same field names, same
/// `EndpointId` type) so we decode exactly what `goose roam id` produced.
#[derive(Deserialize)]
struct ConnectionCard {
    endpoint_id: EndpointId,
    relay_urls: Vec<String>,
}

/// Encode-side mirror of goose-roaming's `ConnectionCard` (same field order and
/// types) so the host decodes our card natively.
#[derive(Serialize)]
struct OwnCard {
    version: u32,
    endpoint_id: EndpointId,
    relay_urls: Vec<String>,
}

fn decode_card(text: &str) -> Result<ConnectionCard, String> {
    use base64::Engine;
    let b64 = text
        .trim()
        .strip_prefix(CARD_SCHEME)
        .ok_or_else(|| format!("missing {CARD_SCHEME} scheme"))?;
    // Same engine goose-roaming's card.rs encodes with, so decode is symmetric.
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|e| format!("decode card base64: {e}"))?;
    let mut card: ConnectionCard =
        serde_json::from_slice(&json).map_err(|e| format!("decode card: {e}"))?;
    // The native card emits relay URLs in FQDN form (`https://host./`). Browsers
    // reject a trailing-dot host in the TLS SNI, so `wss://host./relay` fails
    // with ERR_CONNECTION_CLOSED even though the relay is fine. Normalize here so
    // *both* the relay map and the dial EndpointAddr use the same no-dot host and
    // rendezvous on the same relay.
    for url in &mut card.relay_urls {
        *url = strip_trailing_dot_host(url);
    }
    Ok(card)
}

/// Strip a trailing dot from the host component of an `http(s)://` URL
/// (`https://host./path` -> `https://host/path`), preserving an optional port.
fn strip_trailing_dot_host(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (rest, None),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h, Some(p)),
        _ => (authority, None),
    };
    let host = host.strip_suffix('.').unwrap_or(host);
    let authority = match port {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    };
    match path {
        Some(p) => format!("{scheme}://{authority}/{p}"),
        None => format!("{scheme}://{authority}"),
    }
}

/// Decode a `goose+roam://` card and return its endpoint id (the host's public
/// key) as a string, without dialing. Lets a UI validate a pasted card and show
/// who it's about to connect to. Also the cleanest thing to assert in a browser
/// test: exercises base64 + serde + iroh `EndpointId` parsing in wasm.
#[wasm_bindgen(js_name = decodeCardEndpointId)]
pub fn decode_card_endpoint_id(card_text: String) -> Result<String, JsValue> {
    let card = decode_card(&card_text).map_err(js_err)?;
    Ok(card.endpoint_id.to_string())
}

#[derive(Serialize)]
struct ClientHello {
    label: Option<String>,
}

/// Matches `goose-roaming::handshake::HostAck` (externally-tagged enum).
#[derive(Deserialize)]
enum HostAck {
    Accepted { agent_id: String },
    Rejected { code: String },
}

async fn write_frame<W: futures::io::AsyncWrite + Unpin>(
    w: &mut W,
    body: &[u8],
) -> Result<(), String> {
    if body.len() as u64 > MAX_FRAME_BYTES as u64 {
        return Err("handshake frame too large".into());
    }
    w.write_all(&(body.len() as u32).to_le_bytes())
        .await
        .map_err(|e| e.to_string())?;
    w.write_all(body).await.map_err(|e| e.to_string())?;
    w.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn read_frame<R: futures::io::AsyncRead + Unpin>(r: &mut R) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await.map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME_BYTES {
        return Err("handshake frame too large".into());
    }
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body).await.map_err(|e| e.to_string())?;
    Ok(body)
}

fn relay_mode_from_urls(urls: &[String]) -> RelayMode {
    let configs: Vec<_> = urls
        .iter()
        .filter_map(|u| u.parse().ok())
        .map(|url| RelayConfig::new(url, None))
        .collect();
    if configs.is_empty() {
        RelayMode::Default
    } else {
        RelayMode::Custom(RelayMap::from_iter(configs))
    }
}

/// A roaming identity + the ability to dial a host. Persist the secret (see
/// `secret_hex`) in IndexedDB so the browser keeps a stable key the host can
/// `accept` once.
#[wasm_bindgen]
pub struct RoamClient {
    secret: SecretKey,
}

#[wasm_bindgen]
impl RoamClient {
    /// Create a client. Pass a 64-char hex secret to restore a persisted
    /// identity, or `undefined`/`null` to generate a fresh one.
    #[wasm_bindgen(constructor)]
    pub fn new(secret_hex: Option<String>) -> Result<RoamClient, JsValue> {
        console_error_panic_hook::set_once();
        let secret = match secret_hex {
            Some(hex) if !hex.is_empty() => {
                let bytes = decode_hex_key(&hex).map_err(js_err)?;
                SecretKey::from_bytes(&bytes)
            }
            _ => SecretKey::generate(),
        };
        Ok(RoamClient { secret })
    }

    /// This client's public key (its iroh endpoint id) as a string — the value
    /// the host runs `goose roam peers accept <key>` on.
    #[wasm_bindgen(js_name = endpointId)]
    pub fn endpoint_id(&self) -> String {
        self.secret.public().to_string()
    }

    /// The secret key as hex, for persisting in IndexedDB. Treat as sensitive:
    /// it is this browser's roam identity.
    #[wasm_bindgen(js_name = secretHex)]
    pub fn secret_hex(&self) -> String {
        encode_hex_key(&self.secret.to_bytes())
    }

    /// This browser's identity as a `goose+roam://` card (its public key, with
    /// empty relay URLs — the host never dials the browser back, it only needs
    /// the key on its allowlist). This is what the user hands to the host:
    /// `goose roam peers accept <card>`. Encoded identically to
    /// goose-roaming's `ConnectionCard`, so the host decodes it natively.
    #[wasm_bindgen(js_name = myCard)]
    pub fn my_card(&self) -> Result<String, JsValue> {
        use base64::Engine;
        let card = OwnCard {
            version: 1,
            endpoint_id: self.secret.public(),
            relay_urls: Vec::new(),
        };
        let json = serde_json::to_vec(&card).map_err(|e| js_err(e.to_string()))?;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json);
        Ok(format!("{CARD_SCHEME}{b64}"))
    }

    /// Dial a host from its `goose+roam://` card, run the roam handshake, and
    /// return an authorized [`RoamConnection`] carrying the ACP byte stream.
    #[wasm_bindgen]
    pub async fn connect(
        &self,
        card_text: String,
        label: Option<String>,
    ) -> Result<RoamConnection, JsValue> {
        let card = decode_card(&card_text).map_err(js_err)?;
        console_log!(
            "roam: dialing {} via {} relay(s)",
            card.endpoint_id,
            card.relay_urls.len()
        );

        let ep = Endpoint::builder(Minimal)
            .secret_key(self.secret.clone())
            .relay_mode(relay_mode_from_urls(&card.relay_urls))
            .bind()
            .await
            .map_err(|e| js_err(format!("bind endpoint: {e}")))?;

        let mut addr = EndpointAddr::new(card.endpoint_id);
        for url in &card.relay_urls {
            let parsed = url
                .parse()
                .map_err(|_| js_err(format!("bad relay url {url}")))?;
            addr.addrs.insert(TransportAddr::Relay(parsed));
        }

        let conn = ep
            .connect(addr, ROAMING_ACP_ALPN)
            .await
            .map_err(|e| js_err(format!("connect: {e}")))?;
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| js_err(format!("open_bi: {e}")))?;
        let mut send = send.compat_write();
        let mut recv = recv.compat();

        let hello = serde_json::to_vec(&ClientHello { label }).map_err(|e| js_err(e.to_string()))?;
        write_frame(&mut send, &hello).await.map_err(js_err)?;
        let ack_bytes = read_frame(&mut recv).await.map_err(js_err)?;
        let ack: HostAck =
            serde_json::from_slice(&ack_bytes).map_err(|e| js_err(format!("decode ack: {e}")))?;
        let agent_id = match ack {
            HostAck::Accepted { agent_id } => agent_id,
            HostAck::Rejected { code } => {
                return Err(js_err(format!("rejected by host: {code}")));
            }
        };
        let peer_id = conn.remote_id().to_string();
        console_log!("roam: accepted by `{agent_id}` ({peer_id})");

        // Bridge the iroh stream halves to bounded channels via background
        // tasks so the JS-facing send/recv only touch channels (no borrow of
        // the stream held across a JS await) and get natural backpressure.
        let (to_host_tx, to_host_rx) = async_channel::bounded::<Vec<u8>>(CHANNEL_CAP);
        let (from_host_tx, from_host_rx) = async_channel::bounded::<Vec<u8>>(CHANNEL_CAP);

        spawn_local(async move {
            while let Ok(chunk) = to_host_rx.recv().await {
                if send.write_all(&chunk).await.is_err() || send.flush().await.is_err() {
                    break;
                }
            }
            let _ = send.close().await;
        });
        spawn_local(async move {
            let mut buf = vec![0u8; READ_CHUNK];
            loop {
                match recv.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if from_host_tx.send(buf[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(RoamConnection {
            agent_id,
            peer_id,
            to_host_tx,
            from_host_rx,
            _endpoint: ep,
            _conn: conn,
        })
    }
}

/// An authorized, live connection to a remote agent. `send`/`recv` carry raw
/// ACP bytes; wrap them in Web Streams on the JS side.
#[wasm_bindgen]
pub struct RoamConnection {
    agent_id: String,
    peer_id: String,
    to_host_tx: async_channel::Sender<Vec<u8>>,
    from_host_rx: async_channel::Receiver<Vec<u8>>,
    // Kept alive for the life of the connection; dropping either tears the
    // QUIC session down.
    _endpoint: Endpoint,
    _conn: Connection,
}

#[wasm_bindgen]
impl RoamConnection {
    /// The host-facing id of the agent on the other end.
    #[wasm_bindgen(js_name = agentId)]
    pub fn agent_id(&self) -> String {
        self.agent_id.clone()
    }

    /// The authenticated remote endpoint id (the host's public key).
    #[wasm_bindgen(js_name = peerId)]
    pub fn peer_id(&self) -> String {
        self.peer_id.clone()
    }

    /// Send ACP bytes to the host. Awaits when the outbound buffer is full
    /// (backpressure).
    #[wasm_bindgen]
    pub async fn send(&self, data: Vec<u8>) -> Result<(), JsValue> {
        self.to_host_tx
            .send(data)
            .await
            .map_err(|_| js_err("connection closed".to_string()))
    }

    /// Receive the next chunk of ACP bytes from the host, or `null` once the
    /// stream closes.
    #[wasm_bindgen]
    pub async fn recv(&self) -> Result<Option<Vec<u8>>, JsValue> {
        Ok(self.from_host_rx.recv().await.ok())
    }

    /// Close the outbound half (ends the session cleanly).
    #[wasm_bindgen]
    pub fn close(&self) {
        self.to_host_tx.close();
    }
}

fn js_err(msg: String) -> JsValue {
    JsValue::from_str(&msg)
}

fn encode_hex_key(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn decode_hex_key(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("secret must be 64 hex chars, got {}", hex.len()));
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| "secret not utf-8".to_string())?;
        bytes[i] = u8::from_str_radix(s, 16).map_err(|_| "secret has invalid hex".to_string())?;
    }
    Ok(bytes)
}
