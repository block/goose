//! Builder and runtime handle for an embedded goose agent.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use async_stream::try_stream;
use futures::StreamExt;
use tracing::warn;

use goose::agents::extension::ExtensionConfig;
use goose::agents::{Agent, AgentConfig, AgentEvent, GoosePlatform, SessionConfig};
use goose::config::permission::PermissionManager;
use goose::config::GooseMode;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::providers::create_with_named_model;
use goose::recipe::Recipe;
use goose::session::session_manager::{SessionManager, SessionType};

use crate::handle::ReplyStream;
use crate::permission::{confirmation_for, AutoApprove, PermissionDecider, PermissionRequest};

/// Handle to an embedded goose agent ready to receive prompts.
///
/// Construct with [`Goose::builder`]. Drive with [`Goose::reply`].
pub struct Goose {
    agent: Arc<Agent>,
    session_id: String,
    decider: Arc<dyn PermissionDecider>,
    max_turns: Option<u32>,
}

impl Goose {
    pub fn builder() -> GooseBuilder {
        GooseBuilder::default()
    }

    /// Send a prompt and stream the agent's events.
    ///
    /// The returned stream yields [`AgentEvent`]s as the agent produces them.
    /// Tool-confirmation requests are intercepted: the configured
    /// [`PermissionDecider`] is consulted and the decision is delivered back
    /// to the agent automatically. The originating event is still yielded so
    /// the caller can observe what happened.
    ///
    /// Elicitation requests are not supported in v0 — they're logged as a
    /// warning and the agent stalls waiting for input that never arrives. If
    /// you need elicitations, supply a custom permission decider and handle
    /// the event yourself by holding the stream.
    pub async fn reply(&self, prompt: impl Into<String>) -> Result<ReplyStream<'_>> {
        let user_message = Message::user().with_text(prompt.into());
        let session_config = SessionConfig {
            id: self.session_id.clone(),
            schedule_id: None,
            max_turns: self.max_turns,
            retry_config: None,
        };

        let inner = self.agent.reply(user_message, session_config, None).await?;
        let agent = Arc::clone(&self.agent);
        let decider = Arc::clone(&self.decider);

        let wrapped = try_stream! {
            tokio::pin!(inner);
            while let Some(event) = inner.next().await {
                let event = event?;
                if let AgentEvent::Message(message) = &event {
                    if let Some(request) = find_tool_confirmation(message) {
                        let request_id = request.request_id.clone();
                        let permission = decider.decide(request).await;
                        agent
                            .handle_confirmation(request_id, confirmation_for(permission))
                            .await;
                    } else if contains_elicitation(message) {
                        warn!(
                            "Elicitation requested but goose-embed v0 has no elicitation handler; agent will stall waiting for input"
                        );
                    }
                }
                yield event;
            }
        };

        Ok(ReplyStream {
            inner: Box::pin(wrapped),
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn agent(&self) -> &Arc<Agent> {
        &self.agent
    }
}

fn find_tool_confirmation(message: &Message) -> Option<PermissionRequest> {
    message.content.iter().find_map(|content| {
        if let MessageContent::ActionRequired(action) = content {
            if let ActionRequiredData::ToolConfirmation {
                id,
                tool_name,
                prompt,
                ..
            } = &action.data
            {
                return Some(PermissionRequest {
                    request_id: id.clone(),
                    tool_name: tool_name.clone(),
                    security_prompt: prompt.clone(),
                });
            }
        }
        None
    })
}

fn contains_elicitation(message: &Message) -> bool {
    message.content.iter().any(|content| {
        if let MessageContent::ActionRequired(action) = content {
            matches!(&action.data, ActionRequiredData::Elicitation { .. })
        } else {
            false
        }
    })
}

/// Builder for [`Goose`]. Construct via [`Goose::builder`].
#[derive(Default)]
pub struct GooseBuilder {
    provider_name: Option<String>,
    provider_model: Option<String>,
    extensions: Vec<ExtensionConfig>,
    recipe: Option<Recipe>,
    working_dir: Option<PathBuf>,
    session_name: Option<String>,
    decider: Option<Arc<dyn PermissionDecider>>,
    max_turns: Option<u32>,
}

impl GooseBuilder {
    /// Pick the LLM provider and model. Both must be set before
    /// [`build`](Self::build) unless a recipe with `settings.goose_provider`
    /// and `settings.goose_model` is supplied via [`recipe`](Self::recipe).
    pub fn provider(mut self, name: impl Into<String>, model: impl Into<String>) -> Self {
        self.provider_name = Some(name.into());
        self.provider_model = Some(model.into());
        self
    }

    pub fn extension(mut self, config: ExtensionConfig) -> Self {
        self.extensions.push(config);
        self
    }

    pub fn extensions(mut self, configs: impl IntoIterator<Item = ExtensionConfig>) -> Self {
        self.extensions.extend(configs);
        self
    }

    /// Attach a recipe. The recipe's extensions are added to the agent and
    /// its response schema (if any) is installed. If the builder hasn't set a
    /// provider/model explicitly, the recipe's `settings.goose_provider` and
    /// `settings.goose_model` are used.
    pub fn recipe(mut self, recipe: Recipe) -> Self {
        self.recipe = Some(recipe);
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn session_name(mut self, name: impl Into<String>) -> Self {
        self.session_name = Some(name.into());
        self
    }

    /// Install a [`PermissionDecider`]. Defaults to
    /// [`AutoApprove`](crate::AutoApprove) when omitted.
    pub fn permission_decider<D>(mut self, decider: D) -> Self
    where
        D: PermissionDecider + 'static,
    {
        self.decider = Some(Arc::new(decider));
        self
    }

    pub fn max_turns(mut self, n: u32) -> Self {
        self.max_turns = Some(n);
        self
    }

    pub async fn build(mut self) -> Result<Goose> {
        let (provider_name, provider_model) = resolve_provider(&self)?;

        let provider = create_with_named_model(&provider_name, &provider_model, vec![])
            .await
            .with_context(|| {
                format!("constructing provider '{provider_name}' with model '{provider_model}'")
            })?;

        let agent_config = AgentConfig::new(
            Arc::new(SessionManager::instance()),
            PermissionManager::instance(),
            None,
            GooseMode::Auto,
            true,
            GoosePlatform::GooseCli,
        );
        let agent = Arc::new(Agent::with_config(agent_config));

        let working_dir = self
            .working_dir
            .take()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        let session_name = self
            .session_name
            .take()
            .unwrap_or_else(|| format!("goose-embed-{}", std::process::id()));

        let session = agent
            .config
            .session_manager
            .create_session(
                working_dir,
                session_name,
                SessionType::Hidden,
                GooseMode::Auto,
            )
            .await
            .context("creating embed session")?;

        agent
            .update_provider(provider, &session.id)
            .await
            .context("setting agent provider")?;

        let mut all_extensions = Vec::new();
        if let Some(recipe) = self.recipe.take() {
            agent.apply_recipe_components(recipe.response, false).await;
            if let Some(recipe_extensions) = recipe.extensions {
                all_extensions.extend(recipe_extensions);
            }
        }
        all_extensions.extend(self.extensions);

        for ext in all_extensions {
            let name = ext.name();
            agent
                .add_extension(ext, &session.id)
                .await
                .with_context(|| format!("adding extension '{name}'"))?;
        }

        let decider = self
            .decider
            .unwrap_or_else(|| Arc::new(AutoApprove) as Arc<dyn PermissionDecider>);

        Ok(Goose {
            agent,
            session_id: session.id,
            decider,
            max_turns: self.max_turns,
        })
    }
}

fn resolve_provider(builder: &GooseBuilder) -> Result<(String, String)> {
    let mut name = builder.provider_name.clone();
    let mut model = builder.provider_model.clone();

    if let Some(recipe) = &builder.recipe {
        if let Some(settings) = &recipe.settings {
            if name.is_none() {
                name = settings.goose_provider.clone();
            }
            if model.is_none() {
                model = settings.goose_model.clone();
            }
        }
    }

    match (name, model) {
        (Some(n), Some(m)) => Ok((n, m)),
        (None, _) => {
            bail!("provider name is required: call .provider(name, model) or supply a recipe with settings.goose_provider")
        }
        (_, None) => {
            bail!("model name is required: call .provider(name, model) or supply a recipe with settings.goose_model")
        }
    }
}
