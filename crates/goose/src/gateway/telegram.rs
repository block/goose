use super::{
    Gateway, GatewayConfig, GatewayHandler, IncomingMessage, OutgoingMessage, PlatformUser,
};
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
const POLL_TIMEOUT_SECS: u64 = 30;
const MAX_MESSAGE_LENGTH: usize = 4096;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
/// Maximum voice file size we'll attempt to download (20 MB, Telegram's bot API limit).
const MAX_VOICE_FILE_SIZE: i64 = 20 * 1024 * 1024;

struct VoiceTempFile {
    _file: tempfile::NamedTempFile,
    created_at: std::time::SystemTime,
}

struct VoiceTempFiles {
    parent: PathBuf,
    files: Mutex<Vec<VoiceTempFile>>,
}

impl VoiceTempFiles {
    fn new_in(parent: impl Into<PathBuf>) -> Self {
        Self {
            parent: parent.into(),
            files: Mutex::new(Vec::new()),
        }
    }

    fn save(&self, bytes: &[u8], extension: &str) -> io::Result<PathBuf> {
        let mut file = tempfile::Builder::new()
            .prefix("goose_voice_")
            .suffix(&format!(".{extension}"))
            .tempfile_in(&self.parent)?;
        file.write_all(bytes)?;
        let path = file.path().to_path_buf();
        self.files
            .lock()
            .map_err(|_| io::Error::other("Telegram voice file registry is unavailable"))?
            .push(VoiceTempFile {
                _file: file,
                created_at: std::time::SystemTime::now(),
            });
        Ok(path)
    }

    fn cleanup(&self, max_age: std::time::Duration) -> io::Result<u32> {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(max_age)
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let mut files = self
            .files
            .lock()
            .map_err(|_| io::Error::other("Telegram voice file registry is unavailable"))?;
        let previous_count = files.len();
        files.retain(|file| file.created_at > cutoff);
        Ok((previous_count - files.len()) as u32)
    }

    #[cfg(test)]
    fn parent(&self) -> &std::path::Path {
        &self.parent
    }
}

pub struct TelegramGateway {
    bot_token: String,
    client: Client,
    api_base: String,
    voice_temp_files: Arc<VoiceTempFiles>,
}

#[derive(Debug, Serialize)]
struct SendRichMessageRequest<'a> {
    chat_id: i64,
    rich_message: InputRichMessage<'a>,
}

#[derive(Debug, Serialize)]
struct InputRichMessage<'a> {
    markdown: &'a str,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
    voice: Option<TelegramVoice>,
    audio: Option<TelegramAudio>,
}

#[derive(Debug, Deserialize)]
struct TelegramVoice {
    file_id: String,
    #[allow(dead_code)]
    duration: Option<i32>,
    #[allow(dead_code)]
    mime_type: Option<String>,
    file_size: Option<i64>,
}

/// Audio files sent as documents (not inline voice notes).
#[derive(Debug, Deserialize)]
struct TelegramAudio {
    file_id: String,
    #[allow(dead_code)]
    duration: Option<i32>,
    #[allow(dead_code)]
    mime_type: Option<String>,
    file_size: Option<i64>,
}

/// Metadata extracted from a Telegram voice note or audio attachment.
struct VoiceInfo<'a> {
    file_id: &'a str,
    file_size: Option<i64>,
    duration: Option<i32>,
    mime_type: Option<&'a str>,
}

/// Response from the Telegram `getFile` API.
#[derive(Debug, Deserialize)]
struct TelegramFile {
    #[allow(dead_code)]
    file_id: String,
    file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    first_name: String,
    last_name: Option<String>,
    #[allow(dead_code)]
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    chat_type: String,
}

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

impl TelegramGateway {
    pub fn new(config: &GatewayConfig) -> anyhow::Result<Self> {
        Self::new_with_voice_temp_parent(config, std::env::temp_dir())
    }

    fn new_with_voice_temp_parent(
        config: &GatewayConfig,
        voice_temp_parent: PathBuf,
    ) -> anyhow::Result<Self> {
        let bot_token = config.platform_config["bot_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing bot_token in platform_config"))?
            .to_string();

        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .http1_only()
            .build()?;
        Ok(Self {
            bot_token,
            client,
            api_base: TELEGRAM_API_BASE.to_string(),
            voice_temp_files: Arc::new(VoiceTempFiles::new_in(voice_temp_parent)),
        })
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base, self.bot_token, method)
    }

    async fn send_request(request: RequestBuilder) -> reqwest::Result<Response> {
        request.send().await.map_err(reqwest::Error::without_url)
    }

    async fn response_json<T: DeserializeOwned>(response: Response) -> reqwest::Result<T> {
        response.json().await.map_err(reqwest::Error::without_url)
    }

    async fn response_bytes(response: Response) -> reqwest::Result<Vec<u8>> {
        response
            .bytes()
            .await
            .map(Vec::from)
            .map_err(reqwest::Error::without_url)
    }

    async fn get_updates(&self, offset: Option<i64>) -> anyhow::Result<Vec<TelegramUpdate>> {
        let mut params = serde_json::json!({
            "timeout": POLL_TIMEOUT_SECS,
            "allowed_updates": ["message"],
        });
        if let Some(offset) = offset {
            params["offset"] = serde_json::json!(offset);
        }

        let response = Self::send_request(
            self.client
                .post(self.api_url("getUpdates"))
                .json(&params)
                .timeout(std::time::Duration::from_secs(POLL_TIMEOUT_SECS + 10)),
        )
        .await?;
        let resp: TelegramResponse<Vec<TelegramUpdate>> = Self::response_json(response).await?;

        resp.result.ok_or_else(|| {
            anyhow::anyhow!(
                "Telegram API error: {}",
                resp.description.unwrap_or_default()
            )
        })
    }

    async fn send_text(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let chunks = split_message(text, MAX_MESSAGE_LENGTH);
        for (index, chunk) in chunks.iter().enumerate() {
            let resp = Self::send_request(self.client.post(self.api_url("sendRichMessage")).json(
                &SendRichMessageRequest {
                    chat_id,
                    rich_message: InputRichMessage { markdown: chunk },
                },
            ))
            .await?;

            if let Ok(body) = Self::response_json::<TelegramResponse<serde_json::Value>>(resp).await
            {
                if !body.ok {
                    tracing::warn!(
                        error = body.description.as_deref().unwrap_or("unknown"),
                        "Telegram rejected rich markdown, falling back to plain text"
                    );
                    for plain_chunk in &chunks[index..] {
                        let plain_response =
                            Self::send_request(self.client.post(self.api_url("sendMessage")).json(
                                &serde_json::json!({
                                    "chat_id": chat_id,
                                    "text": plain_chunk,
                                }),
                            ))
                            .await?;
                        let plain_resp: TelegramResponse<serde_json::Value> =
                            Self::response_json(plain_response).await?;
                        if !plain_resp.ok {
                            anyhow::bail!(
                                "Telegram sendMessage failed: {}",
                                plain_resp.description.unwrap_or_default()
                            );
                        }
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn send_chat_action(&self, chat_id: i64, action: &str) -> anyhow::Result<()> {
        Self::send_request(self.client.post(self.api_url("sendChatAction")).json(
            &serde_json::json!({
                "chat_id": chat_id,
                "action": action,
            }),
        ))
        .await?;
        Ok(())
    }

    /// Download a file from Telegram by its `file_id`.
    ///
    /// This is a two-step process:
    /// 1. Call `getFile` to obtain the server-side `file_path`.
    /// 2. Fetch the raw bytes from `https://api.telegram.org/file/bot<TOKEN>/<file_path>`.
    async fn download_file(&self, file_id: &str) -> anyhow::Result<Vec<u8>> {
        // Step 1 – resolve file_id → file_path
        let response = Self::send_request(
            self.client
                .post(self.api_url("getFile"))
                .json(&serde_json::json!({ "file_id": file_id })),
        )
        .await?;
        let resp: TelegramResponse<TelegramFile> = Self::response_json(response).await?;

        let tg_file = resp.result.ok_or_else(|| {
            anyhow::anyhow!(
                "Telegram getFile error: {}",
                resp.description.unwrap_or_default()
            )
        })?;

        let file_path = tg_file
            .file_path
            .ok_or_else(|| anyhow::anyhow!("Telegram getFile returned no file_path"))?;

        // Step 2 – download raw bytes
        let download_url = format!(
            "{}/file/bot{}/{}",
            TELEGRAM_API_BASE, self.bot_token, file_path
        );
        let response = Self::send_request(self.client.get(&download_url)).await?;
        Ok(Self::response_bytes(response).await?)
    }

    /// Save voice bytes to a temporary file and return the path.
    ///
    /// Files are stored as protected, exclusively created temporary files so
    /// Goose can access them via its shell tools. The extension is derived from
    /// the MIME type when available, falling back to `.ogg` for voice notes.
    ///
    /// On Unix files are created with mode `0600` so other local users cannot
    /// read private voice content.
    fn save_voice_file(&self, bytes: &[u8], mime_type: Option<&str>) -> anyhow::Result<PathBuf> {
        let ext = Self::voice_file_extension(mime_type);
        Ok(self.voice_temp_files.save(bytes, &ext)?)
    }

    fn voice_file_extension(mime_type: Option<&str>) -> String {
        let media_type = mime_type
            .and_then(|mime| mime.split(';').next())
            .map(str::trim)
            .map(str::to_ascii_lowercase);
        let subtype = media_type
            .as_deref()
            .and_then(|mime| mime.strip_prefix("audio/"));

        let Some(subtype) = subtype else {
            return "ogg".to_string();
        };

        match subtype {
            "mpeg" => "mp3".to_string(),
            "mp4" | "x-m4a" => "m4a".to_string(),
            "ogg" => "ogg".to_string(),
            "wav" | "x-wav" | "vnd.wave" => "wav".to_string(),
            other
                if other.len() <= 16
                    && other.bytes().enumerate().all(|(index, byte)| {
                        byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'-' | b'_'))
                    }) =>
            {
                other.to_string()
            }
            _ => "ogg".to_string(),
        }
    }

    /// Build the text prompt that tells Goose about a voice message file.
    fn voice_prompt(
        path: &std::path::Path,
        duration: Option<i32>,
        mime_type: Option<&str>,
    ) -> String {
        let duration_hint = duration
            .map(|d| format!(" (duration: {d}s)"))
            .unwrap_or_default();
        let format_hint = mime_type
            .map(|m| format!(" The file format is {m}."))
            .unwrap_or_default();
        format!(
            "The user sent a voice message{duration_hint}. \
             The audio file is saved at: {}{format_hint}\n\n\
             Please transcribe this audio file using available command-line tools \
             (e.g. whisper, ffmpeg, sox, or any STT utility you can find on this system) \
             and then respond to what the user said. \
             If no transcription tool is available, let the user know and ask them to type their message instead.",
            path.display()
        )
    }

    /// Extract metadata from either a voice note or an audio attachment.
    /// Returns `None` when neither is present.
    fn voice_info(msg: &TelegramMessage) -> Option<VoiceInfo<'_>> {
        if let Some(ref v) = msg.voice {
            return Some(VoiceInfo {
                file_id: &v.file_id,
                file_size: v.file_size,
                duration: v.duration,
                mime_type: v.mime_type.as_deref(),
            });
        }
        if let Some(ref a) = msg.audio {
            return Some(VoiceInfo {
                file_id: &a.file_id,
                file_size: a.file_size,
                duration: a.duration,
                mime_type: a.mime_type.as_deref(),
            });
        }
        None
    }

    fn to_platform_user(tg_msg: &TelegramMessage) -> PlatformUser {
        PlatformUser {
            platform: "telegram".to_string(),
            user_id: tg_msg.chat.id.to_string(),
            display_name: tg_msg.from.as_ref().map(|u| {
                let mut name = u.first_name.clone();
                if let Some(ref last) = u.last_name {
                    name.push(' ');
                    name.push_str(last);
                }
                name
            }),
        }
    }
}

#[async_trait]
impl Gateway for TelegramGateway {
    fn gateway_type(&self) -> &str {
        "telegram"
    }

    async fn start(
        &self,
        handler: GatewayHandler,
        cancel: CancellationToken,
    ) -> anyhow::Result<()> {
        let mut offset: Option<i64> = None;

        tracing::info!("Telegram gateway starting long-poll loop");

        // Spawn a background task that periodically removes stale voice files
        // (older than 1 hour) so they don't accumulate on disk.
        let cleanup_cancel = cancel.clone();
        let voice_temp_files = Arc::clone(&self.voice_temp_files);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                tokio::select! {
                    _ = cleanup_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if let Err(error) = voice_temp_files.cleanup(std::time::Duration::from_secs(3600)) {
                            tracing::warn!(%error, "failed to clean up Telegram voice files");
                        }
                    }
                }
            }
        });

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::info!("Telegram gateway shutting down");
                    break;
                }
                result = self.get_updates(offset) => {
                    match result {
                        Ok(updates) => {
                            for update in updates {
                                offset = Some(update.update_id + 1);

                                let Some(tg_msg) = update.message else {
                                    continue;
                                };

                                // Determine the text to send to the handler.
                                // Voice/audio messages are downloaded, saved to
                                // disk, and converted into a prompt that asks
                                // Goose to transcribe the file using CLI tools.
                                let text = if let Some(voice) = Self::voice_info(&tg_msg) {
                                    // Reject files that exceed the Telegram bot
                                    // download limit.
                                    if voice.file_size.unwrap_or(0) > MAX_VOICE_FILE_SIZE {
                                        tracing::warn!(
                                            file_size = voice.file_size,
                                            "voice file exceeds size limit, skipping"
                                        );
                                        continue;
                                    }

                                    match self.download_file(voice.file_id).await {
                                        Ok(bytes) => match self.save_voice_file(&bytes, voice.mime_type) {
                                            Ok(path) => Self::voice_prompt(&path, voice.duration, voice.mime_type),
                                            Err(e) => {
                                                tracing::error!(
                                                    error = %e,
                                                    "failed to save voice file"
                                                );
                                                continue;
                                            }
                                        },
                                        Err(e) => {
                                            tracing::error!(
                                                error = %e,
                                                "failed to download voice file from Telegram"
                                            );
                                            continue;
                                        }
                                    }
                                } else if let Some(ref t) = tg_msg.text {
                                    t.clone()
                                } else {
                                    // Neither text nor voice — skip.
                                    continue;
                                };

                                let user = Self::to_platform_user(&tg_msg);
                                let incoming = IncomingMessage {
                                    user,
                                    text,
                                    platform_message_id: Some(tg_msg.message_id.to_string()),
                                    attachments: vec![],
                                };

                                let handler = handler.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handler.handle_message(incoming).await {
                                        tracing::error!(error = %e, "error handling Telegram message");
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "Telegram poll error");
                            tokio::time::sleep(RETRY_DELAY).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_message(
        &self,
        user: &PlatformUser,
        message: OutgoingMessage,
    ) -> anyhow::Result<()> {
        let chat_id: i64 = user
            .user_id
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid chat_id: {}", user.user_id))?;

        match message {
            OutgoingMessage::Text { body } => {
                self.send_text(chat_id, &body).await?;
            }
            OutgoingMessage::Typing => {
                self.send_chat_action(chat_id, "typing").await?;
            }
        }

        Ok(())
    }

    async fn validate_config(&self) -> anyhow::Result<()> {
        let response = Self::send_request(self.client.get(self.api_url("getMe"))).await?;
        let resp: TelegramResponse<serde_json::Value> = Self::response_json(response).await?;

        if !resp.ok {
            anyhow::bail!(
                "invalid Telegram bot token: {}",
                resp.description.unwrap_or_default()
            );
        }

        if let Some(result) = &resp.result {
            if let Some(username) = result.get("username").and_then(|v| v.as_str()) {
                tracing::info!(bot = %username, "Telegram bot verified");
            }
        }

        Ok(())
    }
}

#[allow(clippy::string_slice)]
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        let mut cut = max_len;
        while cut > 0 && !remaining.is_char_boundary(cut) {
            cut -= 1;
        }
        if cut == 0 {
            cut = remaining
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
        }

        let split_at = remaining[..cut]
            .rfind('\n')
            .or_else(|| remaining[..cut].rfind(' '))
            .map(|pos| pos + 1)
            .unwrap_or(cut);

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SECRET_BOT_TOKEN: &str = "123456789:AASecret_Telegram_Token";

    fn test_gateway(api_base: String) -> TelegramGateway {
        TelegramGateway {
            bot_token: "test-token".to_string(),
            client: Client::builder().no_proxy().build().unwrap(),
            api_base,
            voice_temp_files: Arc::new(VoiceTempFiles::new_in(std::env::temp_dir())),
        }
    }

    fn secret_gateway(api_base: String) -> TelegramGateway {
        TelegramGateway {
            bot_token: SECRET_BOT_TOKEN.to_string(),
            client: Client::builder().no_proxy().build().unwrap(),
            api_base,
            voice_temp_files: Arc::new(VoiceTempFiles::new_in(std::env::temp_dir())),
        }
    }

    fn gateway_with_voice_temp_files(voice_temp_files: VoiceTempFiles) -> TelegramGateway {
        TelegramGateway {
            bot_token: "test-token".to_string(),
            client: Client::builder().no_proxy().build().unwrap(),
            api_base: TELEGRAM_API_BASE.to_string(),
            voice_temp_files: Arc::new(voice_temp_files),
        }
    }

    fn assert_log_fields_are_redacted(
        error: &(impl std::fmt::Display + std::fmt::Debug),
        diagnostic: &str,
    ) {
        let display_log_field = format!("{error}");
        let debug_log_field = format!("{error:?}");

        for rendered in [&display_log_field, &debug_log_field] {
            assert!(!rendered.contains(SECRET_BOT_TOKEN), "{rendered}");
        }
        assert!(
            display_log_field.contains(diagnostic),
            "{display_log_field}"
        );
    }

    #[tokio::test]
    async fn request_errors_remove_token_url_from_display_and_debug() {
        let server = MockServer::start().await;
        let request_path = format!("/bot{SECRET_BOT_TOKEN}/getMe");
        let redirect_url = format!("{}{request_path}", server.uri());

        Mock::given(method("GET"))
            .and(path(request_path))
            .respond_with(
                ResponseTemplate::new(302).append_header("Location", redirect_url.as_str()),
            )
            .mount(&server)
            .await;

        let error = secret_gateway(server.uri())
            .validate_config()
            .await
            .unwrap_err();

        assert_log_fields_are_redacted(&error, "redirect");
        assert!(
            error
                .downcast_ref::<reqwest::Error>()
                .unwrap()
                .is_redirect()
        );
        assert!(
            error
                .downcast_ref::<reqwest::Error>()
                .unwrap()
                .url()
                .is_none()
        );
    }

    #[tokio::test]
    async fn response_errors_remove_token_url_from_display_and_debug() {
        let server = MockServer::start().await;
        let request_path = format!("/bot{SECRET_BOT_TOKEN}/getUpdates");

        Mock::given(method("POST"))
            .and(path(request_path))
            .respond_with(ResponseTemplate::new(200).set_body_raw("{", "application/json"))
            .mount(&server)
            .await;

        let error = secret_gateway(server.uri())
            .get_updates(None)
            .await
            .unwrap_err();

        assert_log_fields_are_redacted(&error, "decoding response body");
        assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_decode());
        assert!(
            error
                .downcast_ref::<reqwest::Error>()
                .unwrap()
                .url()
                .is_none()
        );
    }

    #[tokio::test]
    async fn timeout_errors_remove_token_url_from_display_and_debug() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _connection = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });

        let gateway = secret_gateway(format!("http://{addr}"));
        let url = gateway.api_url("getMe");
        let error = TelegramGateway::send_request(
            gateway
                .client
                .get(url)
                .timeout(std::time::Duration::from_millis(20)),
        )
        .await
        .unwrap_err();

        assert_log_fields_are_redacted(&error, "error sending request");
        assert!(error.is_timeout());
        assert!(error.url().is_none());
    }

    #[tokio::test]
    async fn body_errors_remove_token_url_from_display_and_debug() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            let _ = socket.read(&mut request).await;
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-length: 32\r\nconnection: close\r\n\r\nshort",
                )
                .await
                .unwrap();
        });

        let gateway = secret_gateway(format!("http://{addr}"));
        let url = format!("http://{addr}/file/bot{SECRET_BOT_TOKEN}/voice.ogg");
        let response = TelegramGateway::send_request(gateway.client.get(url))
            .await
            .unwrap();
        let error = TelegramGateway::response_bytes(response).await.unwrap_err();

        assert_log_fields_are_redacted(&error, "error decoding response body");
        assert!(error.is_decode());
        assert!(error.url().is_none());
    }

    #[tokio::test]
    async fn send_text_uses_rich_markdown() {
        let server = MockServer::start().await;
        let markdown = "| Tool | Status |\n|---|---|\n| **MCP** | `ready` |";

        Mock::given(method("POST"))
            .and(path("/bottest-token/sendRichMessage"))
            .and(body_json(serde_json::json!({
                "chat_id": 123,
                "rich_message": { "markdown": markdown },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {},
            })))
            .expect(1)
            .mount(&server)
            .await;

        test_gateway(server.uri())
            .send_text(123, markdown)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn send_text_falls_back_from_rejected_rich_markdown_chunk() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/bottest-token/sendRichMessage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": false,
                "description": "invalid rich markdown",
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottest-token/sendMessage"))
            .and(body_json(serde_json::json!({
                "chat_id": 123,
                "text": "broken **markdown",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {},
            })))
            .expect(1)
            .mount(&server)
            .await;

        test_gateway(server.uri())
            .send_text(123, "broken **markdown")
            .await
            .unwrap();
    }

    #[test]
    fn split_short_message() {
        let chunks = split_message("hello world", 4096);
        assert_eq!(chunks, vec!["hello world"]);
    }

    #[test]
    fn split_at_newline() {
        let text = format!("{}\n{}", "a".repeat(4000), "b".repeat(200));
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 4001);
        assert_eq!(chunks[1].len(), 200);
    }

    #[test]
    fn split_at_space() {
        let text = format!("{} {}", "a".repeat(4000), "b".repeat(200));
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 4001);
        assert_eq!(chunks[1].len(), 200);
    }

    #[test]
    fn split_no_boundary() {
        let text = "a".repeat(5000);
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 4096);
        assert_eq!(chunks[1].len(), 904);
    }

    #[test]
    fn split_exact_boundary() {
        let text = "a".repeat(4096);
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn split_empty() {
        let chunks = split_message("", 4096);
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn split_multiple_chunks() {
        let text = format!(
            "{}\n{}\n{}",
            "a".repeat(4000),
            "b".repeat(4000),
            "c".repeat(4000)
        );
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn split_multibyte_chars() {
        let text = "🦆".repeat(1025); // 4100 bytes
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), 1024);
        assert_eq!(chunks[1].chars().count(), 1);
    }

    #[test]
    fn voice_info_from_voice_message() {
        let msg = TelegramMessage {
            message_id: 1,
            from: None,
            chat: TelegramChat {
                id: 123,
                chat_type: "private".into(),
            },
            text: None,
            voice: Some(TelegramVoice {
                file_id: "voice_file_123".into(),
                duration: Some(5),
                mime_type: Some("audio/ogg".into()),
                file_size: Some(10000),
            }),
            audio: None,
        };
        let info = TelegramGateway::voice_info(&msg);
        assert!(info.is_some());
        let v = info.unwrap();
        assert_eq!(v.file_id, "voice_file_123");
        assert_eq!(v.file_size, Some(10000));
        assert_eq!(v.duration, Some(5));
        assert_eq!(v.mime_type, Some("audio/ogg"));
    }

    #[test]
    fn voice_info_from_audio_message() {
        let msg = TelegramMessage {
            message_id: 1,
            from: None,
            chat: TelegramChat {
                id: 123,
                chat_type: "private".into(),
            },
            text: None,
            voice: None,
            audio: Some(TelegramAudio {
                file_id: "audio_file_456".into(),
                duration: Some(120),
                mime_type: Some("audio/mpeg".into()),
                file_size: Some(500_000),
            }),
        };
        let info = TelegramGateway::voice_info(&msg);
        assert!(info.is_some());
        let v = info.unwrap();
        assert_eq!(v.file_id, "audio_file_456");
        assert_eq!(v.duration, Some(120));
        assert_eq!(v.mime_type, Some("audio/mpeg"));
    }

    #[test]
    fn voice_info_none_for_text() {
        let msg = TelegramMessage {
            message_id: 1,
            from: None,
            chat: TelegramChat {
                id: 123,
                chat_type: "private".into(),
            },
            text: Some("hello".into()),
            voice: None,
            audio: None,
        };
        assert!(TelegramGateway::voice_info(&msg).is_none());
    }

    #[test]
    fn voice_prefers_voice_over_audio() {
        let msg = TelegramMessage {
            message_id: 1,
            from: None,
            chat: TelegramChat {
                id: 123,
                chat_type: "private".into(),
            },
            text: None,
            voice: Some(TelegramVoice {
                file_id: "voice_wins".into(),
                duration: Some(3),
                mime_type: None,
                file_size: None,
            }),
            audio: Some(TelegramAudio {
                file_id: "audio_loses".into(),
                duration: Some(60),
                mime_type: None,
                file_size: None,
            }),
        };
        let v = TelegramGateway::voice_info(&msg).unwrap();
        assert_eq!(v.file_id, "voice_wins");
    }

    #[test]
    fn voice_prompt_includes_path_and_duration() {
        let path = std::path::PathBuf::from("/tmp/goose_voice/voice_test.ogg");
        let prompt = TelegramGateway::voice_prompt(&path, Some(10), Some("audio/ogg"));
        assert!(prompt.contains("/tmp/goose_voice/voice_test.ogg"));
        assert!(prompt.contains("(duration: 10s)"));
        assert!(prompt.contains("audio/ogg"));
        assert!(prompt.contains("transcribe"));
    }

    #[test]
    fn voice_prompt_without_duration() {
        let path = std::path::PathBuf::from("/tmp/goose_voice/voice_test.ogg");
        let prompt = TelegramGateway::voice_prompt(&path, None, None);
        assert!(!prompt.contains("duration"));
        assert!(prompt.contains("/tmp/goose_voice/voice_test.ogg"));
    }

    #[test]
    fn voice_prompt_with_mp3_mime() {
        let path = std::path::PathBuf::from("/tmp/goose_voice/voice_test.mp3");
        let prompt = TelegramGateway::voice_prompt(&path, Some(60), Some("audio/mpeg"));
        assert!(prompt.contains("audio/mpeg"));
        assert!(!prompt.contains("OGG"));
    }

    #[test]
    fn save_voice_file_creates_file_ogg() {
        let gateway = test_gateway(TELEGRAM_API_BASE.to_string());
        let bytes = b"fake ogg data";
        let path = gateway.save_voice_file(bytes, Some("audio/ogg")).unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().ends_with(".ogg"));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn save_voice_file_creates_file_mp3() {
        let gateway = test_gateway(TELEGRAM_API_BASE.to_string());
        let bytes = b"fake mp3 data";
        let path = gateway.save_voice_file(bytes, Some("audio/mpeg")).unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().ends_with(".mp3"));
    }

    #[test]
    fn save_voice_file_defaults_to_ogg() {
        let gateway = test_gateway(TELEGRAM_API_BASE.to_string());
        let bytes = b"unknown format";
        let path = gateway.save_voice_file(bytes, None).unwrap();
        assert!(path.to_str().unwrap().ends_with(".ogg"));
    }

    #[test]
    fn voice_file_extension_preserves_safe_audio_formats() {
        let cases = [
            (Some("audio/mpeg"), "mp3"),
            (Some("audio/mp4"), "m4a"),
            (Some("audio/x-m4a"), "m4a"),
            (Some("audio/ogg; codecs=opus"), "ogg"),
            (Some("audio/x-wav"), "wav"),
            (Some("audio/vnd.wave"), "wav"),
            (Some("audio/flac"), "flac"),
            (Some("audio/WEBM"), "webm"),
            (Some("Audio/MPEG"), "mp3"),
            (Some("AUDIO/WEBM"), "webm"),
        ];

        for (mime_type, expected) in cases {
            assert_eq!(TelegramGateway::voice_file_extension(mime_type), expected);
        }
    }

    #[test]
    fn voice_file_extension_rejects_filename_syntax() {
        let invalid = [
            None,
            Some("application/ogg"),
            Some("audio/..\\..\\outside"),
            Some("audio/../../outside"),
            Some("audio/ogg:stream"),
            Some("audio/ogg.stream"),
            Some("audio/ogg\nnext"),
            Some("audio/ogg; touch=outside"),
            Some("audio/ogg$HOME"),
            Some("audio/åudio"),
            Some("audio/this-subtype-is-too-long"),
        ];

        for mime_type in invalid {
            assert_eq!(TelegramGateway::voice_file_extension(mime_type), "ogg");
        }
    }

    #[test]
    fn save_voice_file_contains_untrusted_mime_before_pairing() {
        let gateway = test_gateway(TELEGRAM_API_BASE.to_string());
        let bytes = b"unpaired voice data";
        let path = gateway
            .save_voice_file(bytes, Some("audio/..\\..\\outside"))
            .unwrap();

        assert_eq!(path.parent(), Some(gateway.voice_temp_files.parent()));
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("goose_voice_"));
        assert!(filename.ends_with(".ogg"));
        assert!(!filename.chars().any(|c| matches!(c, '/' | '\\' | ':')));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn cleanup_handles_legitimate_voice_files() {
        let gateway = test_gateway(TELEGRAM_API_BASE.to_string());
        let recent_file = gateway
            .save_voice_file(b"recent", Some("audio/ogg"))
            .unwrap();
        assert_eq!(
            gateway
                .voice_temp_files
                .cleanup(std::time::Duration::from_secs(3600))
                .unwrap(),
            0
        );
        assert!(recent_file.exists());

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert_eq!(
            gateway
                .voice_temp_files
                .cleanup(std::time::Duration::ZERO)
                .unwrap(),
            1
        );
        assert!(!recent_file.exists());
    }

    #[cfg(unix)]
    #[test]
    fn precreated_legacy_root_cannot_redirect_voice_save_or_cleanup() {
        let sandbox = tempfile::tempdir().unwrap();
        let fake_tmp = sandbox.path().join("tmp");
        let victim_dir = sandbox.path().join("victim");
        std::fs::create_dir(&fake_tmp).unwrap();
        std::fs::create_dir(&victim_dir).unwrap();
        let victim_file = victim_dir.join("unrelated.txt");
        std::fs::write(&victim_file, b"keep me").unwrap();
        std::os::unix::fs::symlink(&victim_dir, fake_tmp.join("goose_voice")).unwrap();
        let gateway = gateway_with_voice_temp_files(VoiceTempFiles::new_in(&fake_tmp));

        let saved = gateway
            .save_voice_file(b"voice", Some("audio/ogg"))
            .unwrap();
        assert_eq!(saved.parent(), Some(fake_tmp.as_path()));
        assert_ne!(saved.parent(), Some(victim_dir.as_path()));
        gateway
            .voice_temp_files
            .cleanup(std::time::Duration::ZERO)
            .unwrap();

        assert!(victim_file.exists());
    }

    #[cfg(windows)]
    #[test]
    fn replaced_legacy_root_cannot_redirect_voice_save_or_cleanup() {
        let sandbox = tempfile::tempdir().unwrap();
        let fake_tmp = sandbox.path().join("tmp");
        let victim_dir = sandbox.path().join("victim");
        std::fs::create_dir(&fake_tmp).unwrap();
        std::fs::create_dir(&victim_dir).unwrap();
        let victim_file = victim_dir.join("unrelated.txt");
        std::fs::write(&victim_file, b"keep me").unwrap();
        let replaced_root = fake_tmp.join("goose_voice");
        std::fs::create_dir(&replaced_root).unwrap();
        std::fs::write(replaced_root.join("attacker-controlled.txt"), b"keep me").unwrap();
        let gateway = gateway_with_voice_temp_files(VoiceTempFiles::new_in(&fake_tmp));

        let saved = gateway
            .save_voice_file(b"voice", Some("audio/ogg"))
            .unwrap();
        assert_eq!(saved.parent(), Some(fake_tmp.as_path()));
        gateway
            .voice_temp_files
            .cleanup(std::time::Duration::ZERO)
            .unwrap();

        assert_eq!(std::fs::read(&victim_file).unwrap(), b"keep me");
        assert_eq!(
            std::fs::read(replaced_root.join("attacker-controlled.txt")).unwrap(),
            b"keep me"
        );
    }

    #[test]
    fn text_gateway_starts_when_voice_storage_is_unavailable() {
        let sandbox = tempfile::tempdir().unwrap();
        let missing_parent = sandbox.path().join("missing");
        let config = GatewayConfig {
            gateway_type: "telegram".to_string(),
            platform_config: serde_json::json!({"bot_token": "test-token"}),
            max_sessions: 1,
        };

        let gateway = TelegramGateway::new_with_voice_temp_parent(&config, missing_parent.clone())
            .expect("text-only startup must not access voice storage");
        assert!(!missing_parent.exists());
        assert!(
            gateway
                .save_voice_file(b"voice", Some("audio/ogg"))
                .is_err()
        );
    }

    #[test]
    fn split_preserves_content() {
        let text = format!(
            "{} {} {}",
            "a".repeat(3000),
            "b".repeat(3000),
            "c".repeat(3000)
        );
        let chunks = split_message(&text, 4096);
        let reassembled: String = chunks.join("");
        assert_eq!(reassembled, text);
    }
}
