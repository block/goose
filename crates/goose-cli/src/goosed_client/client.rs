use anyhow::{anyhow, Result};
use futures::StreamExt;
use goose::agents::ExtensionConfig;
use goose::conversation::message::Message;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::Permission;
use goose::recipe::Recipe;
use goose::session::Session;
use reqwest::Client;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::discovery::{discover_goosed, record_goosed};
use super::handle::GoosedHandle;
use super::types::*;
use super::utils::{find_available_port, find_goosed_binary, generate_secret, process_sse_buffer};

/// Client for communicating with a goosed server instance.
/// Can either spawn a new goosed process or connect to an existing one.
pub struct GoosedClient {
    base_url: String,
    secret_key: String,
    http: Client,
    process: Option<Child>,
}

impl GoosedClient {
    fn build_goosed_command(
        goosed_path: &std::path::Path,
        working_dir: &str,
        port: u16,
        secret_key: &str,
        env_overrides: &[(String, String)],
    ) -> Command {
        let mut cmd = Command::new(goosed_path);
        cmd.arg("agent")
            .env("GOOSE_PORT", port.to_string())
            .env("GOOSE_SERVER__SECRET_KEY", secret_key)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);

        for (key, value) in env_overrides {
            cmd.env(key, value);
        }

        cmd
    }

    /// Spawn a new goosed process and connect to it.
    pub async fn spawn(working_dir: &str) -> Result<Self> {
        Self::spawn_with_env(working_dir, &[]).await
    }

    pub async fn spawn_with_env(
        working_dir: &str,
        env_overrides: &[(String, String)],
    ) -> Result<Self> {
        let port = find_available_port().await?;
        let secret_key = generate_secret();
        let goosed_path = find_goosed_binary()?;

        let mut cmd =
            Self::build_goosed_command(&goosed_path, working_dir, port, &secret_key, env_overrides);

        let process = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn goosed: {}", e))?;

        let base_url = format!("http://127.0.0.1:{}", port);
        let http = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        let mut client = Self {
            base_url,
            secret_key,
            http,
            process: Some(process),
        };

        client.wait_for_ready().await?;
        Ok(client)
    }

    /// Discover a running goosed instance or spawn a new one.
    /// Persists connection state for future CLI invocations.
    pub async fn spawn_or_discover(working_dir: &str) -> Result<Self> {
        // Try to discover an existing instance
        if let Some((base_url, secret_key)) = discover_goosed().await? {
            tracing::info!("Reusing existing goosed instance");
            return Self::connect(&base_url, &secret_key);
        }

        // No running instance — spawn a new one and record it
        let client = Self::spawn(working_dir).await?;
        if let Some(ref proc) = client.process {
            if let Some(pid) = proc.id() {
                record_goosed(
                    client
                        .base_url
                        .split(':')
                        .next_back()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                    &client.secret_key,
                    pid,
                )?;
            }
        }
        Ok(client)
    }

    /// Connect to an existing goosed instance.
    pub fn connect(base_url: &str, secret_key: &str) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            base_url: base_url.to_string(),
            secret_key: secret_key.to_string(),
            http,
            process: None,
        })
    }

    /// Discover a running goosed instance or spawn a new one.
    ///
    /// If `env_overrides` is non-empty, this always spawns a fresh goosed process (and does not
    /// record it for reuse), since env overrides are per-invocation.
    pub async fn spawn_or_discover_with_env(
        working_dir: &str,
        env_overrides: &[(String, String)],
    ) -> Result<Self> {
        if env_overrides.is_empty() {
            return Self::spawn_or_discover(working_dir).await;
        }

        Self::spawn_with_env(working_dir, env_overrides).await
    }

    pub async fn wait_for_ready(&mut self) -> Result<()> {
        let max_attempts = 100;
        let interval = Duration::from_millis(100);

        for attempt in 1..=max_attempts {
            if let Some(ref mut proc) = self.process {
                if let Ok(Some(status)) = proc.try_wait() {
                    return Err(anyhow!("goosed exited prematurely with status: {}", status));
                }
            }

            match self
                .http
                .get(format!("{}/status", self.base_url))
                .header("X-Secret-Key", &self.secret_key)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                _ => {
                    if attempt == max_attempts {
                        return Err(anyhow!(
                            "goosed failed to become ready after {:.1}s",
                            max_attempts as f64 * interval.as_secs_f64()
                        ));
                    }
                    tokio::time::sleep(interval).await;
                }
            }
        }
        unreachable!()
    }

    pub async fn start_agent(
        &self,
        working_dir: &str,
        recipe: Option<&Recipe>,
        extension_overrides: Option<Vec<ExtensionConfig>>,
    ) -> Result<Session> {
        let resp = self
            .http
            .post(format!("{}/agent/start", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&StartAgentRequest {
                working_dir: working_dir.to_string(),
                recipe: recipe.cloned(),
                recipe_id: None,
                recipe_deeplink: None,
                extension_overrides,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("start_agent failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn resume_agent(&self, session_id: &str) -> Result<Session> {
        let resp = self
            .http
            .post(format!("{}/agent/resume", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ResumeAgentRequest {
                session_id: session_id.to_string(),
                load_model_and_extensions: true,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("resume_agent failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn stop_agent(&self, session_id: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/agent/stop", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&StopAgentRequest {
                session_id: session_id.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("stop_agent failed ({}): {}", status, body));
        }

        Ok(())
    }

    /// Send a user message and receive a stream of SseEvents.
    pub async fn reply(
        &self,
        session_id: &str,
        user_message: Message,
        conversation_so_far: Option<Vec<Message>>,
    ) -> Result<ReceiverStream<Result<SseEvent>>> {
        let (tx, rx) = mpsc::channel(256);

        let resp = self
            .http
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .timeout(Duration::from_secs(600))
            .json(&ChatRequest {
                user_message,
                conversation_so_far,
                session_id: session_id.to_string(),
                recipe_name: None,
                recipe_version: None,
                mode: None,
                plan: None,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("reply failed ({}): {}", status, body));
        }

        let mut byte_stream = resp.bytes_stream();
        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        process_sse_buffer(&mut buffer, &tx).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow!("SSE stream error: {}", e))).await;
                        return;
                    }
                }
            }
            if !buffer.trim().is_empty() {
                process_sse_buffer(&mut buffer, &tx).await;
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Send an elicitation response via /reply.
    /// The server treats this as a normal message that unblocks the waiting tool call.
    pub async fn send_elicitation_response(
        &self,
        session_id: &str,
        response_message: Message,
    ) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ChatRequest {
                user_message: response_message,
                conversation_so_far: None,
                session_id: session_id.to_string(),
                recipe_name: None,
                recipe_version: None,
                mode: None,
                plan: None,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "send_elicitation_response failed ({}): {}",
                status,
                body
            ));
        }

        Ok(())
    }

    pub async fn confirm_tool_action(
        &self,
        session_id: &str,
        tool_id: &str,
        permission: Permission,
    ) -> Result<()> {
        let resp = self
            .http
            .post(format!(
                "{}/action-required/tool-confirmation",
                self.base_url
            ))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ToolConfirmationRequest {
                id: tool_id.to_string(),
                principal_type: PrincipalType::Tool,
                action: permission,
                session_id: session_id.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("confirm_tool_action failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let resp = self
            .http
            .get(format!("{}/sessions/{}", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get_session failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let resp = self
            .http
            .get(format!("{}/sessions", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("list_sessions failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn add_extension(&self, session_id: &str, config: ExtensionConfig) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/agent/add_extension", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&AddExtensionRequest {
                session_id: session_id.to_string(),
                config,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("add_extension failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn remove_extension(&self, session_id: &str, name: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/agent/remove_extension", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&RemoveExtensionRequest {
                name: name.to_string(),
                session_id: session_id.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("remove_extension failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn update_provider(
        &self,
        session_id: &str,
        provider: &str,
        model: Option<&str>,
        context_limit: Option<usize>,
        request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/agent/update_provider", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&UpdateProviderRequest {
                provider: provider.to_string(),
                model: model.map(|s| s.to_string()),
                session_id: session_id.to_string(),
                context_limit,
                request_params,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("update_provider failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn get_tools(&self, session_id: &str) -> Result<Vec<ToolInfoResponse>> {
        let resp = self
            .http
            .get(format!("{}/agent/tools", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .query(&[("session_id", session_id)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get_tools failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn list_prompts(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashMap<String, Vec<PromptResponse>>> {
        let resp = self
            .http
            .get(format!("{}/agent/prompts", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .query(&[("session_id", session_id)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("list_prompts failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn get_prompt(
        &self,
        session_id: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<GetPromptResultResponse> {
        let resp = self
            .http
            .post(format!("{}/agent/prompts/get", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&serde_json::json!({
                "session_id": session_id,
                "name": name,
                "arguments": arguments,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get_prompt failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn restart_agent(
        &self,
        session_id: &str,
    ) -> Result<Vec<ExtensionLoadResultResponse>> {
        let resp = self
            .http
            .post(format!("{}/agent/restart", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&RestartAgentRequest {
                session_id: session_id.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("restart_agent failed ({}): {}", status, body));
        }

        let body: RestartAgentResponseBody = resp.json().await?;
        Ok(body.extension_results)
    }

    pub async fn fork_session(
        &self,
        session_id: &str,
        timestamp: Option<i64>,
        truncate: bool,
        copy: bool,
    ) -> Result<String> {
        let resp = self
            .http
            .post(format!("{}/sessions/{}/fork", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ForkSessionRequest {
                timestamp,
                truncate,
                copy,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("fork_session failed ({}): {}", status, body));
        }

        let body: ForkSessionResponseBody = resp.json().await?;
        Ok(body.session_id)
    }

    pub async fn export_session(&self, session_id: &str) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/sessions/{}/export", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("export_session failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn update_working_dir(&self, session_id: &str, working_dir: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/agent/update_working_dir", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&UpdateWorkingDirRequest {
                session_id: session_id.to_string(),
                working_dir: working_dir.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("update_working_dir failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let resp = self
            .http
            .delete(format!("{}/sessions/{}", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("delete_session failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/sessions/{}/clear", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("clear_session failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn add_message(
        &self,
        session_id: &str,
        message: &goose::conversation::message::Message,
    ) -> Result<()> {
        let resp = self
            .http
            .post(format!(
                "{}/sessions/{}/messages",
                self.base_url, session_id
            ))
            .header("X-Secret-Key", &self.secret_key)
            .json(message)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("add_message failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn create_recipe(&self, session_id: &str) -> Result<goose::recipe::Recipe> {
        let resp = self
            .http
            .post(format!("{}/sessions/{}/recipe", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("create_recipe failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }

    /// Create a lightweight, cloneable handle for use in session methods.
    /// This avoids borrow checker issues when the session needs both
    /// the goosed client and mutable self access.
    pub fn handle(&self) -> GoosedHandle {
        GoosedHandle {
            base_url: self.base_url.clone(),
            secret_key: self.secret_key.clone(),
            http: self.http.clone(),
        }
    }

    // --- ACP /runs endpoints ---

    pub async fn create_run(
        &self,
        agent_name: &str,
        session_id: &str,
        input: Vec<goose::acp_compat::AcpMessage>,
    ) -> Result<goose::acp_compat::AcpRun> {
        let body = goose::acp_compat::RunCreateRequest {
            agent_name: agent_name.to_string(),
            session_id: Some(session_id.to_string()),
            input,
            mode: goose::acp_compat::RunMode::Sync,
            metadata: None,
        };
        let resp = self
            .http
            .post(format!("{}/runs", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("create_run failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn get_run(&self, run_id: &str) -> Result<goose::acp_compat::AcpRun> {
        let resp = self
            .http
            .get(format!("{}/runs/{}", self.base_url, run_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get_run failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<goose::acp_compat::AcpRun> {
        let resp = self
            .http
            .post(format!("{}/runs/{}/cancel", self.base_url, run_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("cancel_run failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn resume_run(
        &self,
        run_id: &str,
        data: serde_json::Value,
    ) -> Result<goose::acp_compat::AcpRun> {
        let body = goose::acp_compat::RunResumeRequest {
            run_id: run_id.to_string(),
            await_resume: goose::acp_compat::AwaitResume {
                data: Some(data),
                metadata: None,
            },
            mode: goose::acp_compat::RunMode::Sync,
        };
        let resp = self
            .http
            .post(format!("{}/runs/{}", self.base_url, run_id))
            .header("X-Secret-Key", &self.secret_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("resume_run failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    pub async fn list_run_events(&self, run_id: &str) -> Result<Vec<goose::acp_compat::AcpEvent>> {
        let resp = self
            .http
            .get(format!("{}/runs/{}/events", self.base_url, run_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("list_run_events failed ({}): {}", status, body));
        }

        resp.json().await.map_err(Into::into)
    }

    /// Create a dummy GoosedClient for testing purposes.
    /// The client is not connected to any real server.
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            base_url: "http://127.0.0.1:0".to_string(),
            secret_key: "test-secret".to_string(),
            http: Client::new(),
            process: None,
        }
    }
}

impl Drop for GoosedClient {
    fn drop(&mut self) {
        if let Some(mut proc) = self.process.take() {
            let _ = proc.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_goosed_command_applies_env_overrides() {
        let cmd = GoosedClient::build_goosed_command(
            std::path::Path::new("goosed"),
            ".",
            1234,
            "secret",
            &[(
                "GOOSE_ORCHESTRATOR_MAX_CONCURRENCY".to_string(),
                "7".to_string(),
            )],
        );

        let mut found = false;
        for (k, v) in cmd.as_std().get_envs() {
            if k == std::ffi::OsStr::new("GOOSE_ORCHESTRATOR_MAX_CONCURRENCY") {
                assert_eq!(v, Some(std::ffi::OsStr::new("7")));
                found = true;
            }
        }
        assert!(found, "expected orchestrator concurrency env var to be set");
    }
}
