use anyhow::Result;
use goose::conversation::message::{Message, MessageContent};
use goose_client::Client;
use goose_server::routes::reply::MessageEvent;
use std::io::Write;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

pub async fn run_headless(
    client: Client,
    session_id: String,
    initial_prompt: String,
) -> Result<()> {
    let cancel_token = CancellationToken::new();

    let cancel_clone = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\nInterrupted");
        cancel_clone.cancel();
    });

    let user_message = Message::user().with_text(&initial_prompt);
    println!("[user] {}", initial_prompt);

    let messages = vec![user_message];
    let mut stream = client.reply(messages, session_id).await?;

    let mut in_text_stream = false;
    let mut seen_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                break;
            }
            result = stream.next() => {
                match result {
                    Some(Ok(event)) => {
                        let done = handle_event(
                            &event,
                            &mut in_text_stream,
                            &mut seen_tool_ids,
                        );
                        if done {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("[error] {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn handle_event(
    event: &MessageEvent,
    in_text_stream: &mut bool,
    seen_tool_ids: &mut std::collections::HashSet<String>,
) -> bool {
    match event {
        MessageEvent::Message { message, .. } => {
            for content in &message.content {
                match content {
                    MessageContent::Text(t) => {
                        if !*in_text_stream {
                            print!("[assistant] ");
                            std::io::stdout().flush().ok();
                            *in_text_stream = true;
                        }
                        print!("{}", t.text);
                        std::io::stdout().flush().ok();
                    }
                    MessageContent::ToolRequest(req) => {
                        if *in_text_stream {
                            println!();
                            *in_text_stream = false;
                        }

                        if let Ok(call) = &req.tool_call {
                            if seen_tool_ids.insert(req.id.clone()) {
                                println!("[tool_call] {}", call.name);
                                if let Some(args) = &call.arguments {
                                    if let Ok(pretty) = serde_json::to_string_pretty(args) {
                                        for line in pretty.lines() {
                                            println!("  {}", line);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MessageContent::ToolResponse(resp) => {
                        if *in_text_stream {
                            println!();
                            *in_text_stream = false;
                        }

                        if seen_tool_ids.insert(format!("resp_{}", resp.id)) {
                            let status = if resp.tool_result.is_ok() {
                                "✓"
                            } else {
                                "✗"
                            };
                            println!("[tool_result] {}", status);

                            match &resp.tool_result {
                                Ok(contents) => {
                                    for content in contents {
                                        if let Some(audience) = content.audience() {
                                            if !audience.contains(&rmcp::model::Role::User) {
                                                continue;
                                            }
                                        }
                                        if let rmcp::model::Content {
                                            raw: rmcp::model::RawContent::Text(text_content),
                                            ..
                                        } = content
                                        {
                                            let text = &text_content.text;
                                            for line in text.lines() {
                                                println!("  {}", line);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("  Error: {:?}", e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        MessageEvent::Finish { .. } => {
            if *in_text_stream {
                println!();
            }
            true
        }
        MessageEvent::Error { error } => {
            if *in_text_stream {
                println!();
            }
            eprintln!("[error] {}", error);
            true
        }
        _ => false,
    }
}
