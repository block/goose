use super::completion::GooseCompleter;
use super::input::InputResult;
use super::{RunMode, Session};
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use std::path::PathBuf;

use crate::session::{input, output};

impl Session {
    /// Start an interactive session, optionally with an initial message
    pub async fn interactive(&mut self, message: Option<String>) -> Result<()> {
        // Process initial message if provided
        if let Some(msg) = message {
            self.process_message(msg).await?;
        }

        // Initialize the completion cache
        self.update_completion_cache().await?;

        let mut editor = self.setup_interactive_editor().await?;
        let history_file = self.get_history_file_path()?;
        self.load_command_history(&mut editor, &history_file);

        // Helper function to save history after commands
        let save_history =
            |editor: &mut rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>| {
                if let Err(err) = editor.save_history(&history_file) {
                    eprintln!("Warning: Failed to save command history: {}", err);
                }
            };

        output::display_greeting();
        loop {
            // Display context usage before each prompt
            self.display_context_usage().await?;

            match input::get_input(&mut editor)? {
                input::InputResult::Message(content) => {
                    save_history(&mut editor);

                    match self.run_mode {
                        RunMode::Normal => {
                            self.handle_normal_mode_message(&content).await?;
                        }
                        RunMode::Plan => {
                            self.handle_plan_mode_message(&content).await?;
                        }
                    }
                }
                input::InputResult::Exit => break,
                input::InputResult::AddExtension(cmd) => {
                    save_history(&mut editor);

                    match self.add_extension(cmd.clone()).await {
                        Ok(_) => output::render_extension_success(&cmd),
                        Err(e) => output::render_extension_error(&cmd, &e.to_string()),
                    }
                }
                input::InputResult::AddBuiltin(names) => {
                    save_history(&mut editor);

                    match self.add_builtin(names.clone()).await {
                        Ok(_) => output::render_builtin_success(&names),
                        Err(e) => output::render_builtin_error(&names, &e.to_string()),
                    }
                }
                input::InputResult::ToggleTheme => {
                    save_history(&mut editor);
                    self.handle_theme_toggle();
                    continue;
                }
                input::InputResult::Retry => continue,
                input::InputResult::ListPrompts(extension) => {
                    save_history(&mut editor);

                    match self.list_prompts(extension).await {
                        Ok(prompts) => output::render_prompts(&prompts),
                        Err(e) => output::render_error(&e.to_string()),
                    }
                }
                input::InputResult::GooseMode(mode) => {
                    save_history(&mut editor);

                    if self.handle_goose_mode_setting(&mode)? {
                        continue;
                    }
                }
                input::InputResult::Plan(options) => {
                    save_history(&mut editor);
                    self.handle_plan_command(options).await?;
                }
                input::InputResult::EndPlan => {
                    self.run_mode = RunMode::Normal;
                    output::render_exit_plan_mode();
                    continue;
                }
                input::InputResult::Clear => {
                    save_history(&mut editor);
                    self.handle_clear_command();
                    continue;
                }
                input::InputResult::PromptCommand(opts) => {
                    save_history(&mut editor);
                    self.handle_prompt_command(opts).await?;
                }
                InputResult::Recipe(filepath_opt) => {
                    save_history(&mut editor);
                    self.handle_recipe_command(filepath_opt).await?;
                    continue;
                }
                InputResult::Summarize => {
                    save_history(&mut editor);
                    self.handle_summarize_command().await?;
                    continue;
                }
            }
        }

        println!(
            "\nClosing session.{}",
            self.session_file
                .as_ref()
                .map(|p| format!(" Recorded to {}", p.display()))
                .unwrap_or_default()
        );
        Ok(())
    }

    /// Set up the interactive editor with completer and configuration
    async fn setup_interactive_editor(
        &self,
    ) -> Result<rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>> {
        let config = rustyline::Config::builder()
            .completion_type(rustyline::CompletionType::Circular)
            .build();
        let mut editor =
            rustyline::Editor::<GooseCompleter, rustyline::history::DefaultHistory>::with_config(
                config,
            )?;

        // Set up the completer with a reference to the completion cache
        let completer = GooseCompleter::new(self.completion_cache_manager.get_cache_ref());
        editor.set_helper(Some(completer));

        Ok(editor)
    }

    /// Get the path to the global history file
    fn get_history_file_path(&self) -> Result<PathBuf> {
        let strategy =
            choose_app_strategy(crate::APP_STRATEGY.clone()).expect("goose requires a home dir");
        let config_dir = strategy.config_dir();
        let history_file = config_dir.join("history.txt");

        // Ensure config directory exists
        if let Some(parent) = history_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        Ok(history_file)
    }

    /// Load command history from the global history file
    fn load_command_history(
        &self,
        editor: &mut rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>,
        history_file: &PathBuf,
    ) {
        if history_file.exists() {
            if let Err(err) = editor.load_history(history_file) {
                eprintln!("Warning: Failed to load command history: {}", err);
            }
        }
    }
}
