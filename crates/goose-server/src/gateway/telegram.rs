use super::{
    Gateway, GatewayConfig, GatewayHandler, IncomingMessage, OutgoingMessage, PlatformUser,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
const POLL_TIMEOUT_SECS: u64 = 30;
const MAX_MESSAGE_LENGTH: usize = 4096;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

pub struct TelegramGateway {
    bot_token: String,
    client: Client,
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
        let bot_token = config.platform_config["bot_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing bot_token in platform_config"))?
            .to_string();

        Ok(Self {
            bot_token,
            client: Client::new(),
        })
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", TELEGRAM_API_BASE, self.bot_token, method)
    }

    async fn get_updates(&self, offset: Option<i64>) -> anyhow::Result<Vec<TelegramUpdate>> {
        let mut params = serde_json::json!({
            "timeout": POLL_TIMEOUT_SECS,
            "allowed_updates": ["message"],
        });
        if let Some(offset) = offset {
            params["offset"] = serde_json::json!(offset);
        }

        let resp: TelegramResponse<Vec<TelegramUpdate>> = self
            .client
            .post(self.api_url("getUpdates"))
            .json(&params)
            .timeout(std::time::Duration::from_secs(POLL_TIMEOUT_SECS + 10))
            .send()
            .await?
            .json()
            .await?;

        resp.result.ok_or_else(|| {
            anyhow::anyhow!(
                "Telegram API error: {}",
                resp.description.unwrap_or_default()
            )
        })
    }

    async fn send_text(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let html = super::telegram_format::markdown_to_telegram_html(text);
        for chunk in split_message(&html, MAX_MESSAGE_LENGTH) {
            let resp = self
                .client
                .post(self.api_url("sendMessage"))
                .json(&serde_json::json!({
                    "chat_id": chat_id,
                    "text": chunk,
                    "parse_mode": "HTML",
                }))
                .send()
                .await?;

            if let Ok(body) = resp.json::<TelegramResponse<serde_json::Value>>().await {
                if !body.ok {
                    tracing::warn!(
                        error = body.description.as_deref().unwrap_or("unknown"),
                        "Telegram rejected HTML, falling back to plain text"
                    );
                    for plain_chunk in split_message(text, MAX_MESSAGE_LENGTH) {
                        self.client
                            .post(self.api_url("sendMessage"))
                            .json(&serde_json::json!({
                                "chat_id": chat_id,
                                "text": plain_chunk,
                            }))
                            .send()
                            .await?;
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn send_chat_action(&self, chat_id: i64, action: &str) -> anyhow::Result<()> {
        self.client
            .post(self.api_url("sendChatAction"))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "action": action,
            }))
            .send()
            .await?;
        Ok(())
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
                                let text = match tg_msg.text {
                                    Some(ref t) => t.clone(),
                                    None => continue,
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
                            tracing::error!(error = %e, "Telegram poll error");
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
            OutgoingMessage::Error { message } => {
                self.send_text(chat_id, &format!("⚠️ Error: {}", message))
                    .await?;
            }
            OutgoingMessage::ToolStarted { .. } => {
                self.send_chat_action(chat_id, "typing").await?;
            }
            OutgoingMessage::ToolCompleted { .. } => {}
        }

        Ok(())
    }

    async fn validate_config(&self) -> anyhow::Result<()> {
        let resp: TelegramResponse<serde_json::Value> = self
            .client
            .get(self.api_url("getMe"))
            .send()
            .await?
            .json()
            .await?;

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
