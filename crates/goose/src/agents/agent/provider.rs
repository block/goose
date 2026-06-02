use super::*;

impl Agent {
    pub async fn extend_system_prompt(&self, key: String, instruction: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.add_system_prompt_extra(key, instruction);
    }

    pub async fn set_goal(&self, goal: Option<String>) {
        *self.goal.lock().await = goal;
    }

    pub async fn get_goal(&self) -> Option<String> {
        self.goal.lock().await.clone()
    }

    pub async fn set_grind(&self, goal: Option<String>) {
        *self.grind.lock().await = goal;
    }

    pub async fn get_grind(&self) -> Option<String> {
        self.grind.lock().await.clone()
    }

    pub async fn update_provider(
        &self,
        provider: Arc<dyn Provider>,
        session_id: &str,
    ) -> Result<()> {
        let provider_name = provider.get_name().to_string();
        let model_config = provider.get_model_config();

        let mut current_provider = self.provider.lock().await;
        *current_provider = Some(provider);

        self.config
            .session_manager
            .clone()
            .update(session_id)
            .provider_name(&provider_name)
            .model_config(model_config)
            .apply()
            .await
            .context("Failed to persist provider config to session")
    }

    pub async fn update_goose_mode(&self, mode: GooseMode, session_id: &str) -> Result<()> {
        if let Some(provider) = self.provider.lock().await.as_ref() {
            provider
                .update_mode(session_id, mode)
                .await
                .map_err(|e| anyhow::anyhow!("Provider rejected mode update: {e}"))?;
        }
        *self.current_goose_mode.lock().await = mode;
        self.config
            .session_manager
            .clone()
            .update(session_id)
            .goose_mode(mode)
            .apply()
            .await
            .context("Failed to persist goose_mode to session")
    }

    pub async fn goose_mode(&self) -> GooseMode {
        *self.current_goose_mode.lock().await
    }

    /// Restore the provider from session data or fall back to global config
    /// This is used when resuming a session to restore the provider state
    /// Returns true if the session's provider was replaced with a fallback.
    pub async fn restore_provider_from_session(&self, session: &Session) -> Result<bool> {
        let config = Config::global();

        let provider_name = session
            .provider_name
            .clone()
            .or_else(|| config.get_goose_provider().ok())
            .ok_or_else(|| anyhow!("Could not configure agent: missing provider"))?;

        let model_config = match session.model_config.clone() {
            Some(saved_config) => saved_config,
            None => {
                let model_name = config
                    .get_goose_model()
                    .ok()
                    .ok_or_else(|| anyhow!("Could not configure agent: missing model"))?;
                crate::model::ModelConfig::new(&model_name)
                    .map_err(|e| anyhow!("Could not configure agent: invalid model {}", e))?
                    .with_canonical_limits(&provider_name)
            }
        };

        let extensions =
            EnabledExtensionsState::extensions_or_default(Some(&session.extension_data), config);

        let (provider, provider_changed) = if crate::providers::get_from_registry(&provider_name)
            .await
            .is_ok()
        {
            let p = crate::providers::create(&provider_name, model_config, extensions)
                .await
                .map_err(|e| anyhow!("Could not create provider: {}", e))?;
            (p, false)
        } else {
            let fallback_provider_name = config
                .get_goose_provider()
                .ok()
                .filter(|name| name != &provider_name)
                .ok_or_else(|| {
                    anyhow!(
                        "Could not create provider: provider '{}' not found",
                        provider_name
                    )
                })?;

            tracing::warn!(
                "Session provider '{}' unavailable, falling back to '{}'",
                provider_name,
                fallback_provider_name
            );

            let fallback_model_name = config
                .get_goose_model()
                .ok()
                .ok_or_else(|| anyhow!("Could not configure fallback provider: missing model"))?;
            let fallback_model_config = crate::model::ModelConfig::new(&fallback_model_name)
                .map_err(|e| anyhow!("Could not configure fallback provider: invalid model {}", e))?
                .with_canonical_limits(&fallback_provider_name);

            let fallback_provider = crate::providers::create(
                &fallback_provider_name,
                fallback_model_config.clone(),
                extensions,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not create provider '{}' or fallback '{}': {}",
                    provider_name,
                    fallback_provider_name,
                    e
                )
            })?;

            if let Err(e) = self
                .config
                .session_manager
                .update(&session.id)
                .provider_name(&fallback_provider_name)
                .model_config(fallback_model_config)
                .apply()
                .await
            {
                tracing::warn!("Failed to update session provider: {}", e);
            }

            (fallback_provider, true)
        };

        self.update_provider(provider, &session.id).await?;
        // Propagate session mode to the new provider
        if let Some(provider) = self.provider.lock().await.as_ref() {
            provider
                .update_mode(&session.id, session.goose_mode)
                .await
                .map_err(|e| anyhow!("Failed to propagate mode to provider: {}", e))?;
        }
        *self.current_goose_mode.lock().await = session.goose_mode;
        Ok(provider_changed)
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&self, template: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.set_system_prompt_override(template);
    }

    pub async fn list_extension_prompts(&self, session_id: &str) -> HashMap<String, Vec<Prompt>> {
        self.extension_manager
            .list_prompts(session_id, CancellationToken::default())
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(
        &self,
        session_id: &str,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult> {
        // First find which extension has this prompt
        let prompts = self
            .extension_manager
            .list_prompts(session_id, CancellationToken::default())
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return self
                .extension_manager
                .get_prompt(
                    session_id,
                    extension,
                    name,
                    arguments,
                    CancellationToken::default(),
                )
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self, session_id: &str) -> Result<String> {
        let tools = self
            .extension_manager
            .get_prefixed_tools(session_id, None)
            .await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| {
                ToolInfo::new(
                    &tool.name,
                    tool.description
                        .as_ref()
                        .map(|d| d.as_ref())
                        .unwrap_or_default(),
                    get_parameter_names(&tool),
                    None,
                )
            })
            .collect();

        let plan_prompt = self.extension_manager.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<CallToolResult>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            error!("Failed to send tool result: {}", e);
        }
    }

    pub async fn create_recipe(
        &self,
        session_id: &str,
        mut messages: Conversation,
    ) -> Result<Recipe> {
        tracing::info!("Starting recipe creation with {} messages", messages.len());

        let session = self
            .config
            .session_manager
            .get_session(session_id, false)
            .await?;
        let extensions_info = self
            .extension_manager
            .get_extensions_info(&session.working_dir)
            .await;
        tracing::debug!("Retrieved {} extensions info", extensions_info.len());
        let (extension_count, tool_count) = self.total_extension_and_tool_counts(session_id).await;

        // Get model name from provider
        let provider = self.provider().await.map_err(|e| {
            tracing::error!("Failed to get provider for recipe creation: {}", e);
            e
        })?;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;
        tracing::debug!("Using model: {}", model_name);

        let goose_mode = *self.current_goose_mode.lock().await;
        let prompt_manager = self.prompt_manager.lock().await;
        let system_prompt = prompt_manager
            .builder()
            .with_extensions(extensions_info.into_iter())
            .with_frontend_instructions(self.frontend_instructions.lock().await.clone())
            .with_extension_and_tool_counts(extension_count, tool_count)
            .with_goose_mode(goose_mode)
            .build();

        let recipe_prompt = prompt_manager.get_recipe_prompt().await;
        let tools: Vec<_> = self
            .extension_manager
            .get_prefixed_tools(session_id, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get tools for recipe creation: {}", e);
                e
            })?
            .into_iter()
            .filter(super::super::reply_parts::is_tool_visible_to_model)
            .collect();

        messages.push(Message::user().with_text(recipe_prompt));

        let (messages, issues) = fix_conversation(messages);
        if !issues.is_empty() {
            issues
                .iter()
                .for_each(|issue| tracing::warn!(recipe.conversation.issue = issue));
        }

        tracing::debug!(
            "Added recipe prompt to messages, total messages: {}",
            messages.len()
        );

        tracing::info!("Calling provider to generate recipe content");
        let model_config = {
            let provider_guard = self.provider.lock().await;
            let provider = provider_guard.as_ref().ok_or_else(|| {
                let error = anyhow!("Provider not available during recipe creation");
                tracing::error!("{}", error);
                error
            })?;
            provider.get_model_config()
        };
        let (result, _usage) = self
            .provider
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| {
                let error = anyhow!("Provider not available during recipe creation");
                tracing::error!("{}", error);
                error
            })?
            .complete(
                &model_config,
                session_id,
                &system_prompt,
                messages.messages(),
                &tools,
            )
            .await
            .map_err(|e| {
                tracing::error!("Provider completion failed during recipe creation: {}", e);
                e
            })?;

        let content = result.as_concat_text();
        tracing::debug!(
            "Provider returned content with {} characters",
            content.len()
        );

        // the response may be contained in ```json ```, strip that before parsing json
        let re = Regex::new(r"(?s)```[^\n]*\n(.*?)\n```").unwrap();
        let clean_content = re
            .captures(&content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
            .unwrap_or(&content)
            .trim()
            .to_string();

        let (instructions, activities) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let instructions = json_content
                    .get("instructions")
                    .ok_or_else(|| anyhow!("Missing 'instructions' in json response"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("instructions' is not a string"))?
                    .to_string();

                let activities = json_content
                    .get("activities")
                    .ok_or_else(|| anyhow!("Missing 'activities' in json response"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("'activities' is not an array'"))?
                    .iter()
                    .map(|act| {
                        act.as_str()
                            .map(|s| s.to_string())
                            .ok_or(anyhow!("'activities' array element is not a string"))
                    })
                    .collect::<Result<_, _>>()?;

                (instructions, activities)
            } else {
                tracing::warn!("Failed to parse JSON, falling back to string parsing");
                // If we can't get valid JSON, try string parsing
                // Use split_once to get the content after "Instructions:".
                let after_instructions = content
                    .split_once("instructions:")
                    .map(|(_, rest)| rest)
                    .unwrap_or(&content);

                // Split once more to separate instructions from activities.
                let (instructions_part, activities_text) = after_instructions
                    .split_once("activities:")
                    .unwrap_or((after_instructions, ""));

                let instructions = instructions_part
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '#')
                    .trim()
                    .to_string();
                let activities_text = activities_text.trim();

                // Regex to remove bullet markers or numbers with an optional dot.
                let bullet_re = Regex::new(r"^[•\-*\d]+\.?\s*").expect("Invalid regex");

                // Process each line in the activities section.
                let activities: Vec<String> = activities_text
                    .lines()
                    .map(|line| bullet_re.replace(line, "").to_string())
                    .map(|s| s.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                (instructions, activities)
            };

        let extension_configs = get_enabled_extensions();

        let author = Author {
            contact: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            metadata: None,
        };

        // Ideally we'd get the name of the provider we are using from the provider itself,
        // but it doesn't know and the plumbing looks complicated.
        let config = Config::global();
        let provider_name: String = config
            .get_goose_provider()
            .expect("No provider configured. Run 'goose configure' first");

        let settings = Settings {
            goose_provider: Some(provider_name.clone()),
            goose_model: Some(model_name.clone()),
            temperature: Some(model_config.temperature.unwrap_or(0.0)),
            max_turns: None,
        };

        tracing::debug!(
            "Building recipe with {} activities and {} extensions",
            activities.len(),
            extension_configs.len()
        );

        let (title, description) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let title = json_content
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Custom recipe from chat")
                    .to_string();

                let description = json_content
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("a custom recipe instance from this chat session")
                    .to_string();

                (title, description)
            } else {
                (
                    "Custom recipe from chat".to_string(),
                    "a custom recipe instance from this chat session".to_string(),
                )
            };

        let recipe = Recipe::builder()
            .title(title)
            .description(description)
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .settings(settings)
            .author(author)
            .build()
            .map_err(|e| {
                tracing::error!("Failed to build recipe: {}", e);
                anyhow!("Recipe build failed: {}", e)
            })?;

        tracing::info!("Recipe creation completed successfully");
        Ok(recipe)
    }
}
