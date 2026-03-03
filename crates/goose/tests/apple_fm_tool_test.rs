//! Run with: cargo test --release -p goose --test apple_fm_tool_test -- --nocapture

use fm_rs::{GenerationOptions, Session, SystemLanguageModel, Tool, ToolOutput};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

enum Event {
    Text(String),
    ToolCall {
        args: Value,
        result_tx: oneshot::Sender<String>,
    },
    Done,
    Error(String),
}

struct BridgeShellTool {
    event_tx: mpsc::Sender<Event>,
}

impl Tool for BridgeShellTool {
    fn name(&self) -> &str {
        "shell"
    }
    fn description(&self) -> &str {
        "Execute a shell command and return its output."
    }
    fn arguments_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" }
            },
            "required": ["command"]
        })
    }
    fn call(&self, arguments: Value) -> fm_rs::Result<ToolOutput> {
        let (result_tx, result_rx) = oneshot::channel();
        self.event_tx
            .blocking_send(Event::ToolCall {
                args: arguments,
                result_tx,
            })
            .map_err(|_| fm_rs::Error::GenerationError("channel closed".into()))?;
        result_rx
            .blocking_recv()
            .map_err(|_| fm_rs::Error::GenerationError("result closed".into()))
            .map(ToolOutput::new)
    }
}

async fn run_test(system: &str, prompt: &str, label: &str) {
    let model = match SystemLanguageModel::new() {
        Ok(m) if m.is_available() => m,
        _ => {
            eprintln!("Apple Intelligence not available");
            return;
        }
    };

    let (event_tx, mut event_rx) = mpsc::channel::<Event>(64);
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(BridgeShellTool {
        event_tx: event_tx.clone(),
    })];
    let system = system.to_string();
    let prompt = prompt.to_string();
    let event_tx_clone = event_tx;

    tokio::task::spawn_blocking(move || {
        let session = Session::with_instructions_and_tools(&model, &system, &tools)
            .expect("Failed to create session");
        let options = GenerationOptions::builder().build();
        let tx = event_tx_clone.clone();
        let mut cumulative = String::new();
        let result = session.stream_response(&prompt, &options, move |chunk: &str| {
            if chunk.len() > cumulative.len() {
                let delta: String = chunk.chars().skip(cumulative.chars().count()).collect();
                if !delta.is_empty() {
                    let _ = tx.blocking_send(Event::Text(delta));
                }
                cumulative = chunk.to_string();
            }
        });
        match result {
            Ok(()) => {
                let _ = event_tx_clone.blocking_send(Event::Done);
            }
            Err(e) => {
                let _ = event_tx_clone.blocking_send(Event::Error(e.to_string()));
            }
        }
    });

    eprintln!("\n=== {} ===", label);
    loop {
        match event_rx.recv().await {
            Some(Event::Text(t)) => eprint!("{}", t),
            Some(Event::ToolCall { args, result_tx }) => {
                let cmd = args["command"].as_str().unwrap_or("echo no-command");
                eprintln!("\n[Tool] shell: {}", cmd);
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    .unwrap_or_else(|e| format!("error: {}", e));
                let _ = result_tx.send(output);
            }
            Some(Event::Done) => {
                eprintln!("\n[OK]");
                break;
            }
            Some(Event::Error(e)) => {
                eprintln!("\n[ERROR]: {}", e);
                break;
            }
            None => {
                eprintln!("\n[closed]");
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_short_prompt() {
    run_test(
        "You are a helpful assistant. Use the shell tool to run commands.",
        "List the files in the current directory",
        "SHORT PROMPT",
    )
    .await;
}

#[tokio::test]
async fn test_goose_prompt() {
    let system = "You are a general-purpose AI agent called goose, created by Block, the parent company of Square, CashApp, and Tidal.\n\
        goose is being developed as an open-source software project.\n\n\
        # Extensions\n\n\
        Extensions provide additional tools and context from different data sources and applications.\n\
        You can dynamically enable or disable extensions as needed to help complete tasks.\n\n\
        No extensions are defined. You should let the user know that they should add extensions.\n\n\n\
        # Response Guidelines\n\n\
        Use Markdown formatting for all responses.";
    run_test(
        system,
        "List the files in the current directory",
        "GOOSE PROMPT",
    )
    .await;
}
