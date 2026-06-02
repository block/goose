use super::*;

#[derive(Clone, Serialize, Deserialize, Debug, utoipa::ToSchema)]
pub struct ScheduledJob {
    pub id: String,
    pub source: String,
    pub cron: String,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub currently_running: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub current_session_id: Option<String>,
    #[serde(default)]
    pub process_start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub parameters: Vec<(String, String)>,
    /// Original directory of the recipe file before it was copied to scheduled_recipes/.
    /// Preserved so that relative paths (sub-recipes, template includes) resolve correctly
    /// against the source tree rather than the scheduler's internal storage directory.
    #[serde(default)]
    pub recipe_base_dir: Option<String>,
}

pub async fn persist_jobs(
    storage_path: &Path,
    jobs: &Arc<Mutex<JobsMap>>,
) -> Result<(), SchedulerError> {
    let jobs_guard = jobs.lock().await;
    let list: Vec<ScheduledJob> = jobs_guard.values().map(|(_, j)| j.clone()).collect();
    if let Some(parent) = storage_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(&list)?;
    fs::write(storage_path, data)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn execute_job(
    job: ScheduledJob,
    jobs: Arc<Mutex<JobsMap>>,
    job_id: String,
    cancel_token: CancellationToken,
) -> Result<String> {
    if job.source.is_empty() {
        return Ok(job.id.to_string());
    }

    let recipe_path = Path::new(&job.source);
    let recipe_content = fs::read_to_string(recipe_path)?;
    // Use the original recipe directory for path resolution so that relative
    // references (sub-recipes, template includes) survive the copy into scheduled_recipes/.
    let recipe_dir_owned;
    let recipe_dir = if let Some(ref base) = job.recipe_base_dir {
        recipe_dir_owned = PathBuf::from(base);
        recipe_dir_owned.as_path()
    } else {
        recipe_path.parent().unwrap_or(Path::new("."))
    };

    let recipe: Recipe = build_recipe_from_template(
        recipe_content,
        recipe_dir,
        job.parameters.clone(),
        None::<fn(&str, &str) -> anyhow::Result<String>>,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let agent = Agent::new();

    let config = Config::global();
    let provider_name = config.get_goose_provider()?;
    let model_name = config.get_goose_model()?;
    let model_config =
        crate::model::ModelConfig::new(&model_name)?.with_canonical_limits(&provider_name);

    let session = agent
        .config
        .session_manager
        .create_session(
            std::env::current_dir()?,
            format!("Scheduled job: {}", job.id),
            SessionType::Scheduled,
            agent.config.goose_mode,
        )
        .await?;

    let extensions = resolve_extensions_for_new_session(recipe.extensions.as_deref(), None);
    for ext in &extensions {
        agent.add_extension(ext.clone(), &session.id).await?;
    }

    let agent_provider = create(&provider_name, model_config, extensions).await?;
    agent.update_provider(agent_provider, &session.id).await?;

    let mut jobs_guard = jobs.lock().await;
    if let Some((_, job_def)) = jobs_guard.get_mut(job_id.as_str()) {
        job_def.current_session_id = Some(session.id.clone());
    }
    drop(jobs_guard);

    let start_time = std::time::Instant::now();

    let recipe_display_name = recipe_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&job.id);
    let recipe_version = recipe.version.clone();

    tracing::info!(
        monotonic_counter.goose.session_starts = 1,
        session_type = "schedule",
        interface = "scheduler",
        interactive = false,
        "Scheduled session started"
    );

    tracing::info!(
        monotonic_counter.goose.recipe_runs = 1,
        recipe_name = %recipe_display_name,
        recipe_version = %recipe_version,
        session_type = "schedule",
        interface = "scheduler",
        "Recipe execution started"
    );

    #[cfg(feature = "telemetry")]
    tokio::spawn(async move {
        let mut props = HashMap::new();
        props.insert(
            "trigger".to_string(),
            serde_json::Value::String("automated".to_string()),
        );
        if let Err(e) = posthog::emit_event("schedule_job_started", props).await {
            tracing::debug!("Failed to send schedule telemetry: {}", e);
        }
    });

    let prompt_text = recipe
        .prompt
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            recipe
                .instructions
                .as_deref()
                .filter(|s| !s.trim().is_empty())
        })
        .ok_or_else(|| {
            anyhow!("Recipe must specify at least one of `instructions` or `prompt`.")
        })?;

    let user_message = Message::user().with_text(prompt_text);
    let mut conversation = Conversation::new_unvalidated(vec![user_message.clone()]);

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: Some(job.id.clone()),
        max_turns: None,
        retry_config: None,
    };

    let stream = agent
        .reply(user_message, session_config, Some(cancel_token))
        .await?;

    use futures::StreamExt;
    let mut stream = std::pin::pin!(stream);

    let mut stream_error = false;
    while let Some(message_result) = stream.next().await {
        tokio::task::yield_now().await;

        match message_result {
            Ok(AgentEvent::Message(msg)) => {
                conversation.push(msg);
            }
            Ok(AgentEvent::HistoryReplaced(updated)) => {
                conversation = updated;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error in agent stream: {}", e);
                stream_error = true;
                break;
            }
        }
    }

    agent
        .config
        .session_manager
        .update(&session.id)
        .schedule_id(Some(job.id.clone()))
        .recipe(Some(recipe))
        .apply()
        .await?;

    {
        let session_duration = start_time.elapsed();
        let exit_type = if stream_error { "error" } else { "normal" };
        let (total_tokens, message_count) = agent
            .config
            .session_manager
            .get_session(&session.id, false)
            .await
            .map(|s| (s.total_tokens.unwrap_or(0), s.message_count))
            .unwrap_or((0, 0));

        tracing::info!(
            monotonic_counter.goose.session_completions = 1,
            session_type = "schedule",
            interface = "scheduler",
            exit_type,
            duration_ms = session_duration.as_millis() as u64,
            total_tokens,
            message_count,
            "Session completed"
        );

        tracing::info!(
            monotonic_counter.goose.session_duration_ms = session_duration.as_millis() as u64,
            session_type = "schedule",
            interface = "scheduler",
            "Session duration"
        );

        if total_tokens > 0 {
            tracing::info!(
                monotonic_counter.goose.session_tokens = total_tokens,
                session_type = "schedule",
                interface = "scheduler",
                "Session tokens"
            );
        }
    }

    #[cfg(feature = "telemetry")]
    {
        let duration_secs = start_time.elapsed().as_secs();
        tokio::spawn(async move {
            let mut props = HashMap::new();
            props.insert(
                "trigger".to_string(),
                serde_json::Value::String("automated".to_string()),
            );
            props.insert(
                "status".to_string(),
                serde_json::Value::String("completed".to_string()),
            );
            props.insert(
                "duration_seconds".to_string(),
                serde_json::Value::Number(serde_json::Number::from(duration_secs)),
            );
            if let Err(e) = posthog::emit_event("schedule_job_completed", props).await {
                tracing::debug!("Failed to send schedule telemetry: {}", e);
            }
        });
    }

    Ok(session.id)
}
