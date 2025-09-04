use agent_client_protocol::{self as acp, Client, SessionNotification};
use anyhow::Result;
use goose::agents::Agent;
use goose::config::{Config, ExtensionConfigManager};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::providers::create;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinSet;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Represents a single Goose session for ACP
struct GooseSession {
    agent: Agent,
    messages: Conversation,
}

/// Goose ACP Agent implementation that connects to real Goose agents
struct GooseAcpAgent {
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    sessions: Arc<Mutex<HashMap<String, GooseSession>>>,
    provider: Arc<dyn goose::providers::base::Provider>,
}

impl GooseAcpAgent {
    async fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Result<Self> {
        // Load config and create provider
        let config = Config::global();

        let provider_name: String = config
            .get_param("GOOSE_PROVIDER")
            .map_err(|e| anyhow::anyhow!("No provider configured: {}", e))?;

        let model_name: String = config
            .get_param("GOOSE_MODEL")
            .map_err(|e| anyhow::anyhow!("No model configured: {}", e))?;

        let model_config = goose::model::ModelConfig {
            model_name: model_name.clone(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
        };
        let provider = create(&provider_name, model_config)?;

        Ok(Self {
            session_update_tx,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            provider,
        })
    }
}

impl acp::Agent for GooseAcpAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        info!("ACP: Received initialize request {:?}", arguments);
        Ok(acp::InitializeResponse {
            protocol_version: acp::V1,
            agent_capabilities: acp::AgentCapabilities::default(),
            auth_methods: Vec::new(),
        })
    }

    async fn authenticate(&self, arguments: acp::AuthenticateRequest) -> Result<(), acp::Error> {
        info!("ACP: Received authenticate request {:?}", arguments);
        Ok(())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        info!("ACP: Received new session request {:?}", arguments);

        // Generate a unique session ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create a new Agent and session for this ACP session
        let agent = Agent::new();
        agent
            .update_provider(self.provider.clone())
            .await
            .map_err(|_| acp::Error::internal_error())?;

        // Load and add extensions just like the normal CLI
        // Get all enabled extensions from configuration
        let extensions_to_run: Vec<_> = ExtensionConfigManager::get_all()
            .map_err(|e| {
                error!("Failed to load extensions: {}", e);
                acp::Error::internal_error()
            })?
            .into_iter()
            .filter(|ext| ext.enabled)
            .map(|ext| ext.config)
            .collect();

        // Add extensions to the agent in parallel
        let agent_ptr = Arc::new(agent);
        let mut set = JoinSet::new();
        let mut waiting_on = HashSet::new();

        for extension in extensions_to_run {
            waiting_on.insert(extension.name());
            let agent_ptr_clone = agent_ptr.clone();
            set.spawn(async move {
                (
                    extension.name(),
                    agent_ptr_clone.add_extension(extension.clone()).await,
                )
            });
        }

        // Wait for all extensions to load
        while let Some(result) = set.join_next().await {
            match result {
                Ok((name, Ok(_))) => {
                    waiting_on.remove(&name);
                    info!("Loaded extension: {}", name);
                }
                Ok((name, Err(e))) => {
                    warn!("Failed to load extension '{}': {}", name, e);
                    waiting_on.remove(&name);
                }
                Err(e) => {
                    error!("Task error while loading extension: {}", e);
                }
            }
        }

        // Unwrap the Arc to get the agent back
        let agent = Arc::try_unwrap(agent_ptr).map_err(|_| {
            error!("Failed to unwrap agent Arc");
            acp::Error::internal_error()
        })?;

        let session = GooseSession {
            agent,
            messages: Conversation::new_unvalidated(Vec::new()),
        };

        // Store the session
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);

        info!("Created new session with ID: {}", session_id);

        Ok(acp::NewSessionResponse {
            session_id: acp::SessionId(session_id.into()),
        })
    }

    async fn load_session(&self, arguments: acp::LoadSessionRequest) -> Result<(), acp::Error> {
        info!("ACP: Received load session request {:?}", arguments);
        // For now, we don't support loading previous sessions
        Err(acp::Error::method_not_found())
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        info!("ACP: Received prompt request {:?}", arguments);

        // Get the session
        let session_id = arguments.session_id.0.to_string();
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(acp::Error::invalid_params)?;

        // Convert ACP prompt to Goose message
        // Extract text from ContentBlocks
        let prompt_text = arguments
            .prompt
            .into_iter()
            .filter_map(|block| {
                if let acp::ContentBlock::Text(text) = block {
                    Some(text.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join(" ");

        let user_message = Message::user().with_text(&prompt_text);

        // Add message to conversation
        session.messages.push(user_message);

        // Get agent's reply through the Goose agent
        let cancel_token = CancellationToken::new();
        let mut stream = session
            .agent
            .reply(session.messages.clone(), None, Some(cancel_token.clone()))
            .await
            .map_err(|e| {
                error!("Error getting agent reply: {}", e);
                acp::Error::internal_error()
            })?;

        use futures::StreamExt;

        // Process the agent's response stream
        while let Some(event) = stream.next().await {
            match event {
                Ok(goose::agents::AgentEvent::Message(message)) => {
                    // Add to conversation
                    session.messages.push(message.clone());

                    // Stream the response text to the client
                    for content_item in &message.content {
                        if let MessageContent::Text(text) = content_item {
                            let (tx, rx) = oneshot::channel();
                            self.session_update_tx
                                .send((
                                    SessionNotification {
                                        session_id: arguments.session_id.clone(),
                                        update: acp::SessionUpdate::AgentMessageChunk {
                                            content: text.text.clone().into(),
                                        },
                                    },
                                    tx,
                                ))
                                .map_err(|_| acp::Error::internal_error())?;
                            rx.await.map_err(|_| acp::Error::internal_error())?;
                        }
                    }
                }
                Ok(_) => {
                    // Ignore other events for now
                }
                Err(e) => {
                    error!("Error in agent response stream: {}", e);
                    return Err(acp::Error::internal_error());
                }
            }
        }

        Ok(acp::PromptResponse {
            stop_reason: acp::StopReason::EndTurn,
        })
    }

    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        info!("ACP: Received cancel request {:?}", args);
        Ok(())
    }
}

/// Run the ACP agent server
pub async fn run_acp_agent() -> Result<()> {
    info!("Starting Goose ACP agent server on stdio");
    eprintln!("Goose ACP agent started. Listening on stdio...");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    // The AgentSideConnection will spawn futures onto our Tokio runtime.
    // LocalSet and spawn_local are used because the futures from the
    // agent-client-protocol crate are not Send.
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            // Start up the GooseAcpAgent connected to stdio.
            let agent = GooseAcpAgent::new(tx)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create ACP agent: {}", e))?;
            let (conn, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            // Kick off a background task to send the agent's session notifications to the client.
            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = rx.recv().await {
                    let result = conn.session_notification(session_notification).await;
                    if let Err(e) = result {
                        error!("ACP session notification error: {}", e);
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            // Run until stdin/stdout are closed.
            handle_io.await
        })
        .await?;

    Ok(())
}
