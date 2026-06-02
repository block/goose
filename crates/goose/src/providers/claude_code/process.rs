use super::*;

pub(super) struct CliProcess {
    pub child: tokio::process::Child,
    pub stdin: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    pub reader: BufReader<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
    #[allow(dead_code)]
    pub stderr_handle: tokio::task::JoinHandle<String>,
    pub current_model: String,
    pub log_model_update: bool,
    pub next_request_id: u64,
    pub needs_drain: bool,
}

impl std::fmt::Debug for CliProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliProcess")
            .field("current_model", &self.current_model)
            .field("next_request_id", &self.next_request_id)
            .finish_non_exhaustive()
    }
}

impl CliProcess {
    pub fn next_request_id(&mut self) -> String {
        let id = self.next_request_id;
        self.next_request_id += 1;
        format!("req_{id}")
    }

    pub async fn send_control_request(
        &mut self,
        body: ControlRequestBody,
    ) -> Result<Option<Value>, ProviderError> {
        let request_id = self.next_request_id();
        exchange_control(&mut self.stdin, &mut self.reader, &request_id, body).await
    }

    pub async fn send_set_model(&mut self, model: &str) -> Result<(), ProviderError> {
        if model == self.current_model {
            return Ok(());
        }
        self.send_control_request(ControlRequestBody::SetModel {
            model: model.to_string(),
        })
        .await?;
        self.current_model = model.to_string();
        self.log_model_update = true;
        Ok(())
    }

    pub async fn drain_pending_response(&mut self) {
        if !self.needs_drain {
            return;
        }
        tracing::debug!("Draining cancelled response from CLI process");

        let drain = async {
            let mut line = String::new();
            loop {
                line.clear();
                match self.reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("result") | Some("error") => break,
                                _ => continue,
                            }
                        } else {
                            tracing::trace!(line = trimmed, "Non-JSON line during drain");
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        if tokio::time::timeout(DRAIN_TIMEOUT, drain).await.is_err() {
            // CLI is still producing the old response. Leave needs_drain
            // true so the next call retries — by then the old response
            // likely completed and drain will succeed quickly.
            tracing::warn!(
                "Drain did not complete in {DRAIN_TIMEOUT:?}; \
                 will retry on next request"
            );
            return;
        }

        self.needs_drain = false;
        tracing::debug!("Drain complete, protocol re-synced");
    }
}

impl Drop for CliProcess {
    fn drop(&mut self) {
        self.stderr_handle.abort();
        let _ = self.child.start_kill();
    }
}
