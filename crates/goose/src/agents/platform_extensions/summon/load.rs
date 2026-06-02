use super::*;

impl SummonClient {
    pub(super) async fn handle_load(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        self.cleanup_completed_tasks().await;

        let source_name = arguments
            .as_ref()
            .and_then(|args| args.get("source"))
            .and_then(|v| v.as_str());

        let cancel = arguments
            .as_ref()
            .and_then(|args| args.get("cancel"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let working_dir = self.get_working_dir(session_id).await;

        if source_name.is_none() {
            return self
                .handle_load_discovery(session_id, &working_dir)
                .await
                .map(CallToolResult::success);
        }

        let name = source_name.unwrap();

        if is_session_id(name) {
            let content = self.handle_load_task_result(name, cancel).await?;
            let mut meta = Meta::new();
            meta.0.insert(
                "subagent_session_id".to_string(),
                serde_json::Value::String(name.to_string()),
            );
            return Ok(CallToolResult::success(content).with_meta(Some(meta)));
        }

        self.handle_load_source(session_id, name, &working_dir)
            .await
            .map(CallToolResult::success)
    }

    pub(super) async fn handle_load_task_result(
        &self,
        task_id: &str,
        cancel: bool,
    ) -> Result<Vec<Content>, String> {
        let mut completed = self.completed_tasks.lock().await;

        if let Some(task) = completed.remove(task_id) {
            let status = if task.result.is_ok() {
                "✓ Completed"
            } else {
                "✗ Failed"
            };
            let output = match task.result {
                Ok(output) => output,
                Err(error) => format!("Error: {}", error),
            };

            return Ok(vec![Content::text(format!(
                "# Background Task Result: {}\n\n\
                 **Task:** {}\n\
                 **Status:** {}\n\
                 **Duration:** {} ({} turns)\n\n\
                 ## Output\n\n{}",
                task_id,
                task.description,
                status,
                round_duration(task.duration),
                task.turns_taken,
                output
            ))]);
        }

        drop(completed);

        let mut running = self.background_tasks.lock().await;
        if running.contains_key(task_id) {
            if cancel {
                let task = running.remove(task_id).unwrap();
                drop(running);

                task.cancellation_token.cancel();

                let duration = task.started_at.elapsed();
                let turns_taken = task.turns.load(Ordering::Relaxed);

                let mut handle = task.handle;
                let output = tokio::select! {
                    result = &mut handle => {
                        match result {
                            Ok(Ok(s)) => s,
                            Ok(Err(e)) => format!("Error: {}", e),
                            Err(e) => format!("Task panicked: {}", e),
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        handle.abort();
                        "Task did not stop in time (aborted)".to_string()
                    }
                };

                return Ok(vec![Content::text(format!(
                    "# Background Task Result: {}\n\n\
                     **Task:** {}\n\
                     **Status:** ⊘ Cancelled\n\
                     **Duration:** {} ({} turns)\n\n\
                     ## Output\n\n{}",
                    task_id,
                    task.description,
                    round_duration(duration),
                    turns_taken,
                    output
                ))]);
            }

            // Wait for the running task to complete, keeping the tool call
            // alive so notifications (subagent tool calls) stream in real time.
            let mut task = running.remove(task_id).unwrap();
            drop(running);

            let buffered = {
                let mut buf = task.notification_buffer.lock().await;
                std::mem::take(&mut *buf)
            };
            if !buffered.is_empty() {
                let subs = self.notification_subscribers.lock().await;
                for notif in buffered {
                    for tx in subs.iter() {
                        let _ = tx.try_send(notif.clone());
                    }
                }
            }

            tokio::select! {
                result = &mut task.handle => {
                    let output = match result {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => format!("Error: {}", e),
                        Err(e) => format!("Task panicked: {}", e),
                    };

                    return Ok(vec![Content::text(format!(
                        "# Background Task Result: {}\n\n\
                         **Task:** {}\n\
                         **Status:** ✓ Completed\n\
                         **Duration:** {} ({} turns)\n\n\
                         ## Output\n\n{}",
                        task_id,
                        task.description,
                        round_duration(task.started_at.elapsed()),
                        task.turns.load(Ordering::Relaxed),
                        output
                    ))]);
                }
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    self.background_tasks.lock().await.insert(task_id.to_string(), task);

                    return Err(format!(
                        "Task '{task_id}' is still running after waiting 5 min. \
                         Use load(source: \"{task_id}\") to wait again, or \
                         load(source: \"{task_id}\", cancel: true) to stop."
                    ));
                }
            }
        }

        Err(format!("Task '{}' not found.", task_id))
    }

    pub(super) async fn handle_load_discovery(
        &self,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Vec<Content>, String> {
        {
            let mut cache = self.source_cache.lock().await;
            *cache = None;
        }

        let sources = self.get_sources(session_id, working_dir).await;
        let completed = self.completed_tasks.lock().await;

        if sources.is_empty() && completed.is_empty() {
            return Ok(vec![Content::text(
                "No sources available for load/delegate.\n\n\
                 Sources are discovered from:\n\
                 • Current recipe's sub_recipes\n\
                 • .agents/recipes/, .agents/agents/ (project-level)\n\
                 • ~/.agents/agents/ (global)\n\
                 • GOOSE_RECIPE_PATH directories",
            )]);
        }

        let mut output = String::from("Available sources for load/delegate:\n");

        if !completed.is_empty() {
            output.push_str("\nCompleted Tasks (awaiting retrieval):\n");
            let mut sorted_completed: Vec<_> = completed.values().collect();
            sorted_completed.sort_by_key(|t| &t.id);
            for task in sorted_completed {
                let status = if task.result.is_ok() {
                    "completed"
                } else {
                    "failed"
                };
                output.push_str(&format!(
                    "• {} - \"{}\" ({})\n",
                    task.id, task.description, status
                ));
            }
        }

        for kind in [SourceType::Subrecipe, SourceType::Recipe, SourceType::Agent] {
            let kind_sources: Vec<_> = sources.iter().filter(|s| s.source_type == kind).collect();
            if !kind_sources.is_empty() {
                output.push_str(&format!("\n{}:\n", kind_plural(kind)));
                for source in kind_sources {
                    output.push_str(&format!(
                        "• {} - {}\n",
                        source.name,
                        safe_truncate(&source.description, SUBAGENT_DESCRIPTION_BUDGET)
                    ));
                }
            }
        }

        output.push_str("\nUse load(source: \"name\") to load into context.\n");
        output.push_str("Use delegate(source: \"name\") to run as subagent.");

        Ok(vec![Content::text(output)])
    }

    pub(super) async fn handle_load_source(
        &self,
        session_id: &str,
        name: &str,
        working_dir: &Path,
    ) -> Result<Vec<Content>, String> {
        let source = self.resolve_source(session_id, name, working_dir).await?;

        match source {
            Some(source) => {
                let content = source.to_load_text();

                let output = format!(
                    "# Loaded: {} ({})\n\n{}\n\n---\nThis knowledge is now available in your context.",
                    source.name, source.source_type, content
                );

                Ok(vec![Content::text(output)])
            }
            None => {
                let sources = self.get_sources(session_id, working_dir).await;

                let suggestions: Vec<&str> = sources
                    .iter()
                    .filter(|s| {
                        s.name.to_lowercase().contains(&name.to_lowercase())
                            || name.to_lowercase().contains(&s.name.to_lowercase())
                    })
                    .take(3)
                    .map(|s| s.name.as_str())
                    .collect();

                let error_msg = if suggestions.is_empty() {
                    format!(
                        "Source '{}' not found. Use load() to see available sources.",
                        name
                    )
                } else {
                    format!(
                        "Source '{}' not found. Did you mean: {}?",
                        name,
                        suggestions.join(", ")
                    )
                };

                Err(error_msg)
            }
        }
    }
}
