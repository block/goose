use super::*;

impl Agent {
    pub(super) async fn frontend_extension_configs(&self) -> Vec<ExtensionConfig> {
        let mut configs = self
            .frontend_extensions
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        configs.sort_by_key(|config| config.key());
        configs
    }

    pub(super) async fn frontend_tools_for_extension(
        &self,
        extension_name: Option<&str>,
    ) -> Vec<Tool> {
        let requested_extension = extension_name.map(name_to_key);

        self.frontend_extension_configs()
            .await
            .into_iter()
            .filter_map(|config| {
                let include = requested_extension
                    .as_ref()
                    .is_none_or(|name| *name == config.key());

                match config {
                    ExtensionConfig::Frontend { tools, .. } if include => Some(tools),
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }

    pub(super) async fn rebuild_frontend_derived_state(
        &self,
        extensions: &HashMap<String, ExtensionConfig>,
    ) {
        let multiple = extensions.len() > 1;
        let mut tools = HashMap::new();
        let mut instructions = Vec::new();

        for config in extensions.values() {
            if let ExtensionConfig::Frontend {
                name,
                tools: ext_tools,
                instructions: ext_instructions,
                ..
            } = config
            {
                for tool in ext_tools {
                    let tool_name = tool.name.to_string();
                    tools.insert(
                        tool_name.clone(),
                        FrontendTool {
                            name: tool_name,
                            tool: tool.clone(),
                        },
                    );
                }

                let text = ext_instructions
                    .clone()
                    .unwrap_or_else(|| super::DEFAULT_FRONTEND_INSTRUCTIONS.to_string());
                instructions.push(if multiple {
                    format!("{name}: {text}")
                } else {
                    text
                });
            }
        }

        *self.frontend_tools.lock().await = tools;
        *self.frontend_instructions.lock().await = if instructions.is_empty() {
            None
        } else {
            Some(instructions.join("\n\n"))
        };
    }

    pub(super) async fn insert_frontend_extension(&self, extension: ExtensionConfig) {
        let mut extensions = self.frontend_extensions.lock().await;
        extensions.insert(extension.key(), extension);
        self.rebuild_frontend_derived_state(&extensions).await;
    }

    pub(super) async fn remove_frontend_extension(&self, name: &str) {
        let mut extensions = self.frontend_extensions.lock().await;
        extensions.remove(&name_to_key(name));
        self.rebuild_frontend_derived_state(&extensions).await;
    }

    pub(super) async fn extension_configs_for_persistence(&self) -> Vec<ExtensionConfig> {
        let mut extension_configs = self.extension_manager.get_extension_configs().await;
        extension_configs.extend(self.frontend_extension_configs().await);
        extension_configs
    }

    pub(crate) async fn total_extension_and_tool_counts(&self, session_id: &str) -> (usize, usize) {
        let (extension_count, tool_count) = self
            .extension_manager
            .get_extension_and_tool_counts(session_id)
            .await;

        (
            extension_count + self.frontend_extensions.lock().await.len(),
            tool_count + self.frontend_tools.lock().await.len(),
        )
    }

    /// Save current extension state to session metadata
    /// Should be called after any extension add/remove operation
    pub async fn save_extension_state(&self, session: &SessionConfig) -> Result<()> {
        let extensions_state =
            EnabledExtensionsState::new(self.extension_configs_for_persistence().await);

        let session_manager = self.config.session_manager.clone();
        let mut session_data = session_manager.get_session(&session.id, false).await?;

        if let Err(e) = extensions_state.to_extension_data(&mut session_data.extension_data) {
            warn!("Failed to serialize extension state: {}", e);
            return Err(anyhow!("Extension state serialization failed: {}", e));
        }

        session_manager
            .update(&session.id)
            .extension_data(session_data.extension_data)
            .apply()
            .await?;

        Ok(())
    }

    /// Save current extension state to session by session_id
    pub async fn persist_extension_state(&self, session_id: &str) -> Result<()> {
        let extensions_state =
            EnabledExtensionsState::new(self.extension_configs_for_persistence().await);

        let session_manager = self.config.session_manager.clone();
        let session = session_manager.get_session(session_id, false).await?;
        let mut extension_data = session.extension_data.clone();

        extensions_state
            .to_extension_data(&mut extension_data)
            .map_err(|e| anyhow!("Failed to serialize extension state: {}", e))?;

        session_manager
            .update(session_id)
            .extension_data(extension_data)
            .apply()
            .await?;

        Ok(())
    }

    /// Load extensions from session into the agent
    /// Skips extensions that are already loaded
    /// Uses the session's working_dir for extension initialization
    pub async fn load_extensions_from_session(
        self: &Arc<Self>,
        session: &Session,
    ) -> Vec<ExtensionLoadResult> {
        let session_extensions =
            EnabledExtensionsState::from_extension_data(&session.extension_data);
        let enabled_configs = match session_extensions {
            Some(state) => state.extensions,
            None => {
                tracing::warn!(
                    "No extensions found in session {}. This is unexpected.",
                    session.id
                );
                return vec![];
            }
        };

        let session_id = session.id.clone();

        let extension_futures = enabled_configs
            .into_iter()
            .map(|config| {
                let config_clone = config.clone();
                let agent_ref = self.clone();
                let session_id_clone = session_id.clone();

                async move {
                    let name = config_clone.name().to_string();

                    if agent_ref
                        .extension_manager
                        .is_extension_enabled(&name)
                        .await
                    {
                        tracing::debug!("Extension {} already loaded, skipping", name);
                        return ExtensionLoadResult {
                            name,
                            success: true,
                            error: None,
                        };
                    }

                    match agent_ref
                        .add_extension_inner(config_clone, &session_id_clone)
                        .await
                    {
                        Ok(_) => ExtensionLoadResult {
                            name,
                            success: true,
                            error: None,
                        },
                        Err(e) => {
                            let error_msg = e.to_string();
                            warn!("Failed to load extension {}: {}", name, error_msg);
                            ExtensionLoadResult {
                                name,
                                success: false,
                                error: Some(error_msg),
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(extension_futures).await;

        // Persist once after all extensions are loaded
        if results.iter().any(|r| r.success) {
            if let Err(e) = self.persist_extension_state(&session_id).await {
                warn!("Failed to persist extension state after bulk load: {}", e);
            }
        }

        results
    }

    pub async fn add_extension(
        &self,
        extension: ExtensionConfig,
        session_id: &str,
    ) -> ExtensionResult<()> {
        self.add_extension_inner(extension, session_id).await?;

        // Persist extension state after successful add
        self.persist_extension_state(session_id)
            .await
            .map_err(|e| {
                error!("Failed to persist extension state: {}", e);
                crate::agents::extension::ExtensionError::SetupError(format!(
                    "Failed to persist extension state: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Load multiple extensions in parallel, persisting state once at the end.
    ///
    /// Unlike `add_extension`, this avoids per-extension persistence and acquires
    /// the container lock once upfront to prevent serialisation of the parallel futures.
    pub async fn add_extensions_bulk(
        self: &Arc<Self>,
        extensions: Vec<ExtensionConfig>,
        session_id: &str,
    ) -> anyhow::Result<Vec<ExtensionLoadResult>> {
        let working_dir = match self
            .config
            .session_manager
            .get_session(session_id, false)
            .await
        {
            Ok(session) => Some(session.working_dir),
            Err(e) => {
                warn!("Failed to get session for bulk load: {}", e);
                None
            }
        };
        let container = self.container.lock().await.clone();

        let extension_futures = extensions
            .into_iter()
            .map(|config| {
                let ext_manager = Arc::clone(&self.extension_manager);
                let working_dir = working_dir.clone();
                let container = container.clone();
                let sid = session_id.to_string();

                async move {
                    let name = config.name().to_string();
                    match ext_manager
                        .add_extension(config, working_dir, container.as_ref(), Some(&sid))
                        .await
                    {
                        Ok(_) => ExtensionLoadResult {
                            name,
                            success: true,
                            error: None,
                        },
                        Err(e) => {
                            let error_msg = e.to_string();
                            warn!("Failed to load extension {}: {}", name, error_msg);
                            ExtensionLoadResult {
                                name,
                                success: false,
                                error: Some(error_msg),
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(extension_futures).await;

        if results.iter().any(|r| r.success) {
            self.persist_extension_state(session_id).await?;
        }

        Ok(results)
    }

    pub(super) async fn add_extension_inner(
        &self,
        extension: ExtensionConfig,
        session_id: &str,
    ) -> ExtensionResult<()> {
        let session = self
            .config
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| {
                crate::agents::extension::ExtensionError::SetupError(format!(
                    "Failed to get session '{}': {}",
                    session_id, e
                ))
            })?;
        let working_dir = Some(session.working_dir);

        match &extension {
            ExtensionConfig::Frontend { .. } => {
                self.insert_frontend_extension(extension.clone()).await;
            }
            _ => {
                let container = self.container.lock().await;
                self.extension_manager
                    .add_extension(
                        extension.clone(),
                        working_dir,
                        container.as_ref(),
                        Some(session_id),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn list_tools(&self, session_id: &str, extension_name: Option<String>) -> Vec<Tool> {
        let mut prefixed_tools = self
            .extension_manager
            .get_prefixed_tools(session_id, extension_name.clone())
            .await
            .unwrap_or_default();

        prefixed_tools.extend(
            self.frontend_tools_for_extension(extension_name.as_deref())
                .await,
        );

        if (extension_name.is_none() || extension_name.as_deref() == Some("platform"))
            && self.config.scheduler_service.is_some()
        {
            prefixed_tools.push(platform_tools::manage_schedule_tool());
        }

        if extension_name.is_none() {
            if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                prefixed_tools.push(final_output_tool.tool());
            }
        }

        prefixed_tools
    }

    pub async fn remove_extension(&self, name: &str, session_id: &str) -> Result<()> {
        self.extension_manager.remove_extension(name).await?;
        self.remove_frontend_extension(name).await;

        // Persist extension state after successful removal
        self.persist_extension_state(session_id)
            .await
            .map_err(|e| {
                error!("Failed to persist extension state: {}", e);
                anyhow!("Failed to persist extension state: {}", e)
            })?;

        Ok(())
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let mut extensions = self
            .extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions");
        extensions.extend(
            self.frontend_extension_configs()
                .await
                .into_iter()
                .map(|config| config.name()),
        );
        extensions
    }

    pub async fn get_extension_configs(&self) -> Vec<ExtensionConfig> {
        self.extension_configs_for_persistence().await
    }
}
