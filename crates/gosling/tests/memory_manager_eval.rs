//! Long-conversation memory evaluation for the Context Manager MVP
//! (`GOSLING_CONTEXT_MANAGER`), gosling's fork-specific feature over base
//! goose.
//!
//! Five scenarios, each with a "needle" fact planted early in the
//! conversation and a final question that can only be answered correctly if
//! that fact is still recoverable. Each scenario is built up to a target of
//! several million tokens of synthetic filler (tool calls/responses with
//! large, realistic-shaped content) and evaluated at three context-window
//! sizes representative of real model limits: 128K, 200K, and 1M tokens.
//!
//! This harness exercises `ContextManager::build_with_counter` and
//! `FileMemorySource` directly (both are deterministic and provider-free),
//! not the full `Agent::reply` loop — `context_manager_integration.rs`
//! already covers the end-to-end wiring at small scale; this file is about
//! behavior and robustness at the scale the fork is meant to help with.
//!
//! Heavy (multi-million-token) runs are `#[ignore]`d so normal `cargo test`
//! stays fast; run them explicitly with:
//!   cargo test -p gosling --test memory_manager_eval -- --ignored --nocapture

use gosling::context_mgmt::{
    ContextBuildRequest, ContextManager, ContextManagerMode, FileMemorySource, MemoryQuery,
    MemorySource, NoopMemorySource,
};
use gosling::conversation::message::Message;
use gosling::token_counter::{create_token_counter, TokenCounter};
use rmcp::model::{AnnotateAble, CallToolRequestParams, CallToolResult, RawContent};
use serde::Serialize;
use std::time::Instant;

const WINDOW_SIZES: [(&str, usize); 3] = [("128K", 128_000), ("200K", 200_000), ("1M", 1_000_000)];

/// ~4 characters per token is the standard rough heuristic for English text;
/// used only to size synthetic filler, never for the actual measurements
/// (those go through the real tiktoken-backed counter).
const CHARS_PER_TOKEN_ESTIMATE: usize = 4;

struct Scenario {
    name: &'static str,
    /// The fact planted early that must survive to the end of the conversation.
    needle: &'static str,
    /// Where the needle is embedded, as a full early user message.
    needle_message: &'static str,
    /// The final user question; answering it correctly requires the needle.
    final_question: &'static str,
    /// Deterministic filler content lines, cycled to build bulk tool output.
    filler_lines: &'static [&'static str],
}

const SCENARIOS: [Scenario; 5] = [
    Scenario {
        name: "codebase_refactor_marathon",
        needle: "UUIDv7 primary keys",
        needle_message: "Before we start: architectural decision — every new table we add during this refactor must use UUIDv7 primary keys, not auto-increment ints. Payments module is owned by another team, don't touch it.",
        final_question: "We're about to create the new `shipments` table. What primary key type should it use, and is there anything we should avoid touching?",
        filler_lines: &[
            "diff --git a/src/service.rs b/src/service.rs",
            "-    pub id: i64,",
            "+    pub id: Uuid,",
            "running cargo build ... warning: unused import `std::fmt`",
            "test result: ok. 42 passed; 0 failed; 0 ignored",
            "refactored module boundary between billing and inventory",
            "renamed struct OrderLine -> LineItem across 14 call sites",
        ],
    },
    Scenario {
        name: "incident_response_marathon",
        needle: "request timeout config is set to 30000ms",
        needle_message: "Status update: we already confirmed it's NOT a DNS issue — checked at 14:02 UTC, resolver latency was normal. The service's request timeout config is set to 30000ms, that hasn't changed recently.",
        final_question: "Given everything we've gathered in this incident so far, should we investigate DNS resolution again, and what is the current request timeout config value?",
        filler_lines: &[
            "2026-07-04T09:12:03Z ERROR upstream connection reset by peer",
            "2026-07-04T09:12:04Z WARN retrying request, attempt 3/5",
            "cpu_usage{pod=\"api-7f9\"} 0.82",
            "memory_usage{pod=\"api-7f9\"} 0.61",
            "GET /health 200 12ms",
            "checked load balancer target group: all healthy",
            "reviewed recent deploys: no changes in the last 6 hours",
        ],
    },
    Scenario {
        name: "requirements_gathering_thread",
        needle: "SLA is 99.95% uptime",
        needle_message: "Hard requirement from the client: the mobile app MUST support full offline mode on iOS — this is non-negotiable. Also the contracted SLA is 99.95% uptime.",
        final_question: "Can you summarize the requirements we've gathered for the client proposal, including the mobile and reliability requirements?",
        filler_lines: &[
            "client call notes: discussed dashboard color scheme preferences",
            "searched docs: found reference implementation for push notifications",
            "compared pricing tiers across three competitor products",
            "drafted onboarding email copy, awaiting client feedback",
            "reviewed accessibility requirements (WCAG 2.1 AA)",
            "client asked about timeline for the Android release",
        ],
    },
    Scenario {
        name: "research_literature_review",
        needle: "focus only on transformer-based approaches",
        needle_message: "Scope for this literature review: exclude any papers published before 2020, and focus only on transformer-based approaches — not RNNs or CNNs, even if they're cited by transformer papers.",
        final_question: "Based on everything reviewed, can you compile the shortlist of papers that fit our review's scope?",
        filler_lines: &[
            "Abstract: We propose a novel recurrent architecture for sequence modeling (2018)...",
            "Abstract: A convolutional approach to language understanding (2016)...",
            "Abstract: Scaling attention mechanisms for long-context reasoning (2023)...",
            "citation graph: 214 papers reference this work",
            "extracted benchmark table: BLEU scores across 12 datasets",
            "noted: this paper's ablation study lacks a controlled baseline",
        ],
    },
    Scenario {
        name: "personal_planning_assistant",
        needle: "allergic to shellfish",
        needle_message: "A few things about me you should remember for this whole planning process: I'm allergic to shellfish, and the total budget cap for this trip is $5,000 — please don't suggest anything that would blow that.",
        final_question: "Given everything we've planned so far, what restaurant should we book for the final night, and does it fit within what I told you earlier?",
        filler_lines: &[
            "flight search: found 3 options with layovers under 2 hours",
            "hotel comparison: reviewed ratings and cancellation policies",
            "itinerary draft: day 1 museum, day 2 hiking, day 3 free time",
            "currency conversion notes and tipping customs for the destination",
            "packing list draft: weather forecast shows mild temperatures",
            "checked visa requirements: none needed for this itinerary",
        ],
    },
];

/// Generates deterministic but genuinely unique filler text for tool
/// response `index`. The absolute chunk counter (`index * 1_000 + i`, not
/// just `i`) is what guarantees uniqueness across different `index` values —
/// an earlier version reset the counter to 0 on every call and only rotated
/// which of the ~7 flavor lines came first, so most tool outputs ended up
/// byte-identical to another one (only 7 distinct rotations existed) and the
/// Context Manager's duplicate-tool-output dedup silently absorbed nearly
/// all of the growth before real budget-driven summarization ever triggered.
fn filler_block(scenario: &Scenario, index: usize, target_chars: usize) -> String {
    let mut text = String::with_capacity(target_chars + 128);
    let mut i = 0usize;
    while text.len() < target_chars {
        let line = scenario.filler_lines[(index + i) % scenario.filler_lines.len()];
        let absolute_chunk = index * 1_000 + i;
        text.push_str(line);
        text.push_str(&format!(" [msg {index} chunk {absolute_chunk}]\n"));
        i += 1;
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NeedlePosition {
    /// Needle is one of the first messages in the conversation. Realistic
    /// for a stated-up-front constraint, but also the position most flattered
    /// by `summarize_group`'s preview-from-the-front behavior (see below).
    Start,
    /// Needle is planted after roughly half the filler has already
    /// accumulated — the realistic case for a fact stated mid-session (a
    /// decision made partway through a long debugging/refactor marathon).
    Middle,
}

fn filler_pair(scenario: &Scenario, index: usize, per_message_chars: usize) -> [Message; 2] {
    let call_id = format!("call_{index}");
    let request = Message::assistant()
        .with_tool_request(call_id.as_str(), Ok(CallToolRequestParams::new("run_step")));
    let content = filler_block(scenario, index, per_message_chars);
    let response = Message::user().with_tool_response(
        call_id.as_str(),
        Ok(CallToolResult::success(vec![
            RawContent::text(content).no_annotation()
        ])),
    );
    [request, response]
}

/// Builds a synthetic long conversation: needle message at `position`, bulk
/// filler as tool call/response pairs (large tool outputs, few messages —
/// keeps per-message tokenization cost bounded even at multi-million-token
/// scale), then the final question as the very last message.
fn build_conversation(
    scenario: &Scenario,
    target_tokens: usize,
    position: NeedlePosition,
) -> Vec<Message> {
    let mut messages = Vec::new();
    messages.push(Message::user().with_text("Starting a new work session."));
    messages.push(Message::assistant().with_text("Ready to help. What are we working on?"));

    let target_chars = target_tokens * CHARS_PER_TOKEN_ESTIMATE;
    let per_message_chars = 12_000usize; // ~3K tokens per tool response
    let num_pairs = (target_chars / per_message_chars).max(1);

    let needle_at = match position {
        NeedlePosition::Start => 0,
        NeedlePosition::Middle => num_pairs / 2,
    };

    for i in 0..num_pairs {
        if i == needle_at {
            messages.push(Message::user().with_text(scenario.needle_message));
            messages.push(
                Message::assistant().with_text("Understood, I'll keep that in mind throughout."),
            );
        }
        messages.extend(filler_pair(scenario, i, per_message_chars));
    }
    if needle_at >= num_pairs {
        messages.push(Message::user().with_text(scenario.needle_message));
        messages
            .push(Message::assistant().with_text("Understood, I'll keep that in mind throughout."));
    }

    messages.push(Message::user().with_text(scenario.final_question));
    messages
}

fn conversation_text(messages: &[Message]) -> String {
    messages
        .iter()
        .flat_map(|m| m.content.iter())
        .filter_map(|c| c.as_text())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Serialize)]
struct EvalResult {
    scenario: String,
    needle_position: String,
    window: String,
    context_limit: usize,
    mode: String,
    memory_seeded: bool,
    input_tokens_estimated: usize,
    tokens_before: usize,
    tokens_after: usize,
    strategy: String,
    dropped_block_count: usize,
    summarized_block_count: usize,
    needle_recalled_in_final_input: bool,
    retrieved_memory_used: bool,
    build_ms: u128,
    panicked: bool,
    error: Option<String>,
}

#[allow(clippy::too_many_arguments)]
async fn eval_one(
    token_counter: &TokenCounter,
    scenario: &Scenario,
    target_tokens: usize,
    needle_position: NeedlePosition,
    window_name: &str,
    context_limit: usize,
    mode: ContextManagerMode,
    memory_seeded: bool,
) -> EvalResult {
    let messages = build_conversation(scenario, target_tokens, needle_position);
    let input_tokens_estimated = messages.len();

    let memory_dir = tempfile::tempdir().expect("tempdir");
    let memory_path = memory_dir.path().join("memories.jsonl");
    if memory_seeded {
        std::fs::write(
            &memory_path,
            format!(
                r#"{{"content": {:?}, "source": "session:earlier"}}"#,
                scenario.needle_message
            ),
        )
        .expect("seed memory file");
    }

    let retrieved_memory = if memory_seeded {
        let source = FileMemorySource::new(memory_path.clone());
        let query = MemoryQuery {
            session_id: scenario.name,
            messages: &messages,
            reserved_tokens: (context_limit as f64 * 0.10) as usize,
        };
        source.retrieve(&query)
    } else {
        NoopMemorySource.retrieve(&MemoryQuery {
            session_id: scenario.name,
            messages: &messages,
            reserved_tokens: 0,
        })
    };
    let retrieved_memory_used = !retrieved_memory.is_empty();

    let start = Instant::now();
    let build = std::panic::AssertUnwindSafe(|| {
        ContextManager::build_with_counter(
            ContextBuildRequest {
                system_prompt: "You are a helpful software engineering agent.".to_string(),
                project_instructions: None,
                conversation_messages: messages.clone(),
                context_limit,
                reserved_response_tokens: 4_096,
                retrieved_memory,
            },
            token_counter,
        )
    });
    let result = std::panic::catch_unwind(build);
    let build_ms = start.elapsed().as_millis();

    match result {
        Ok(packet) => {
            let (_, final_messages) = gosling::context_mgmt::resolve_provider_input(
                mode,
                &packet,
                "You are a helpful software engineering agent.",
                &messages,
            );
            let final_text = conversation_text(&final_messages);
            let needle_recalled = final_text.contains(scenario.needle)
                || final_text.contains(scenario.needle_message);

            EvalResult {
                scenario: scenario.name.to_string(),
                needle_position: format!("{:?}", needle_position),
                window: window_name.to_string(),
                context_limit,
                mode: format!("{:?}", mode),
                memory_seeded,
                input_tokens_estimated,
                tokens_before: packet.metadata.estimated_tokens_before,
                tokens_after: packet.metadata.estimated_tokens_after,
                strategy: packet.metadata.strategy.as_str().to_string(),
                dropped_block_count: packet.metadata.dropped_blocks.iter().map(|d| d.count).sum(),
                summarized_block_count: packet.metadata.summarized_blocks.len(),
                needle_recalled_in_final_input: needle_recalled,
                retrieved_memory_used,
                build_ms,
                panicked: false,
                error: None,
            }
        }
        Err(e) => EvalResult {
            scenario: scenario.name.to_string(),
            needle_position: format!("{:?}", needle_position),
            window: window_name.to_string(),
            context_limit,
            mode: format!("{:?}", mode),
            memory_seeded,
            input_tokens_estimated,
            tokens_before: 0,
            tokens_after: 0,
            strategy: "n/a".to_string(),
            dropped_block_count: 0,
            summarized_block_count: 0,
            needle_recalled_in_final_input: false,
            retrieved_memory_used,
            build_ms,
            panicked: true,
            error: Some(format!("{:?}", e)),
        },
    }
}

/// Full sweep: 5 scenarios x 2 needle positions x 3 context windows x
/// {on/no-memory, on/with-memory, shadow/no-memory} = 90 runs, each against a
/// conversation sized to ~2.5M tokens of synthetic content (see module docs
/// re: message-count vs. token-count scaling tradeoffs).
#[tokio::test]
#[ignore]
async fn eval_all_scenarios_context_manager() {
    let token_counter = create_token_counter().await.expect("token counter");
    let target_tokens = 2_500_000usize;

    let mut results = Vec::new();
    let mut any_panicked = false;

    for scenario in SCENARIOS.iter() {
        for needle_position in [NeedlePosition::Start, NeedlePosition::Middle] {
            for (window_name, context_limit) in WINDOW_SIZES.iter() {
                for (mode, memory_seeded) in [
                    (ContextManagerMode::On, false),
                    (ContextManagerMode::On, true),
                    (ContextManagerMode::Shadow, false),
                ] {
                    let result = eval_one(
                        &token_counter,
                        scenario,
                        target_tokens,
                        needle_position,
                        window_name,
                        *context_limit,
                        mode,
                        memory_seeded,
                    )
                    .await;
                    any_panicked |= result.panicked;
                    println!(
                        "[{}] needle={} window={} mode={} memory_seeded={} strategy={} tokens_before={} tokens_after={} needle_recalled={} retrieved_memory_used={} build_ms={} panicked={}",
                        result.scenario,
                        result.needle_position,
                        result.window,
                        result.mode,
                        result.memory_seeded,
                        result.strategy,
                        result.tokens_before,
                        result.tokens_after,
                        result.needle_recalled_in_final_input,
                        result.retrieved_memory_used,
                        result.build_ms,
                        result.panicked,
                    );
                    results.push(result);
                }
            }
        }
    }

    let json = serde_json::to_string_pretty(&results).expect("serialize results");
    let out_path = std::env::var("MEMORY_EVAL_OUTPUT").unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("gosling2_context_manager_eval_results.json")
            .to_string_lossy()
            .into_owned()
    });
    std::fs::write(&out_path, &json).expect("write results file");
    println!("Wrote {} results to {}", results.len(), out_path);

    assert!(
        !any_panicked,
        "Context Manager must not panic at multi-million-token scale"
    );
}

/// The recall sweep above plants the needle as its own plain-text user
/// message, which `classify_blocks` groups into a small, isolated
/// `OlderConversationSummary` run distinct from the surrounding tool-heavy
/// traffic — `summarize_group`'s 600-character preview then covers it in
/// full, so it always survives regardless of position. That is a real and
/// useful property, but it isn't the whole picture: the realistic case where
/// a fact is mentioned *inside* a large tool result (a config value visible
/// in a build log, a decision noted partway through a long file) puts the
/// needle inside a multi-thousand-token `SummarizedToolResults` block, fully
/// subject to the same 600-character-from-the-front preview truncation. This
/// test demonstrates that failure mode directly, and shows the file-backed
/// memory source is unaffected by it (recall there is independent of where
/// in the conversation — or in what shape — the fact originally appeared).
#[tokio::test]
#[ignore]
async fn needle_buried_inside_tool_output_is_lost_without_memory_seed() {
    let token_counter = create_token_counter().await.expect("token counter");
    let scenario = &SCENARIOS[1]; // incident_response_marathon
    let target_tokens = 2_500_000usize;
    let per_message_chars = 12_000usize;
    let target_chars = target_tokens * CHARS_PER_TOKEN_ESTIMATE;
    let num_pairs = (target_chars / per_message_chars).max(1);
    let needle_pair_index = num_pairs / 2;

    let mut messages = vec![
        Message::user().with_text("Starting a new work session."),
        Message::assistant().with_text("Ready to help. What are we working on?"),
    ];
    for i in 0..num_pairs {
        if i == needle_pair_index {
            // The needle is buried mid-way through an otherwise-ordinary
            // tool output, exactly as a config value would appear in the
            // middle of a real log dump — not as its own message.
            let mut content = filler_block(scenario, i, per_message_chars / 2);
            content.push_str(&format!("\nNOTE: {}\n", scenario.needle_message));
            content.push_str(&filler_block(scenario, i + 1, per_message_chars / 2));
            let call_id = format!("call_{i}");
            messages.push(
                Message::assistant().with_tool_request(
                    call_id.as_str(),
                    Ok(CallToolRequestParams::new("run_step")),
                ),
            );
            messages.push(Message::user().with_tool_response(
                call_id.as_str(),
                Ok(CallToolResult::success(vec![
                    RawContent::text(content).no_annotation(),
                ])),
            ));
        } else {
            messages.extend(filler_pair(scenario, i, per_message_chars));
        }
    }
    messages.push(Message::user().with_text(scenario.final_question));

    // Without a seeded memory file: does the buried needle survive the
    // Context Manager's packet-building at a realistic (128K) window?
    let packet_no_memory = ContextManager::build_with_counter(
        ContextBuildRequest {
            system_prompt: "system".to_string(),
            project_instructions: None,
            conversation_messages: messages.clone(),
            context_limit: 128_000,
            reserved_response_tokens: 4_096,
            retrieved_memory: Vec::new(),
        },
        &token_counter,
    );
    let text_no_memory = conversation_text(&packet_no_memory.messages);
    let recalled_no_memory = text_no_memory.contains(scenario.needle);

    // With the same fact seeded into the file-backed memory source (as if
    // an operator or the agent itself had written it down earlier): recall
    // no longer depends on where the fact sat in the raw conversation.
    let memory_dir = tempfile::tempdir().expect("tempdir");
    let memory_path = memory_dir.path().join("memories.jsonl");
    std::fs::write(
        &memory_path,
        format!(
            r#"{{"content": {:?}, "source": "session:earlier"}}"#,
            scenario.needle_message
        ),
    )
    .expect("seed memory file");
    let source = FileMemorySource::new(memory_path);
    let retrieved_memory = source.retrieve(&MemoryQuery {
        session_id: scenario.name,
        messages: &messages,
        reserved_tokens: 12_800,
    });
    let packet_with_memory = ContextManager::build_with_counter(
        ContextBuildRequest {
            system_prompt: "system".to_string(),
            project_instructions: None,
            conversation_messages: messages,
            context_limit: 128_000,
            reserved_response_tokens: 4_096,
            retrieved_memory,
        },
        &token_counter,
    );
    let text_with_memory = conversation_text(&packet_with_memory.messages);
    let recalled_with_memory =
        text_with_memory.contains(scenario.needle) || text_with_memory.contains("session:earlier");

    println!(
        "buried needle recall: without_memory_seed={recalled_no_memory} with_memory_seed={recalled_with_memory}"
    );

    assert!(
        !recalled_no_memory,
        "expected the buried needle to be lost to preview truncation without a memory seed \
         (if this now passes, the summarization behavior changed — re-check the finding)"
    );
    assert!(
        recalled_with_memory,
        "file-backed memory should recover the fact regardless of where it appeared in the raw conversation"
    );
}

/// Stress/robustness checks distinct from the recall sweep above: adversarial
/// inputs that should degrade gracefully, never panic or hang.
#[tokio::test]
#[ignore]
async fn stress_single_giant_tool_output_does_not_panic() {
    let token_counter = create_token_counter().await.expect("token counter");
    let huge = "x ".repeat(2_000_000); // ~2M tokens in one message
    let messages = vec![
        Message::user().with_text("read this huge file"),
        Message::assistant()
            .with_tool_request("call1", Ok(CallToolRequestParams::new("read_file"))),
        Message::user().with_tool_response(
            "call1",
            Ok(CallToolResult::success(vec![
                RawContent::text(huge).no_annotation()
            ])),
        ),
        Message::user().with_text("what did it say?"),
    ];

    let build = std::panic::AssertUnwindSafe(|| {
        ContextManager::build_with_counter(
            ContextBuildRequest {
                system_prompt: "system".to_string(),
                project_instructions: None,
                conversation_messages: messages,
                context_limit: 128_000,
                reserved_response_tokens: 4_096,
                retrieved_memory: Vec::new(),
            },
            &token_counter,
        )
    });
    let result = std::panic::catch_unwind(build);
    assert!(result.is_ok(), "single giant tool output must not panic");
}

#[test]
fn stress_large_memory_file_recall_is_fast_and_bounded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("memories.jsonl");
    let mut contents = String::new();
    for i in 0..20_000 {
        contents.push_str(&format!(
            r#"{{"content": "fact number {i} about widgets and databases", "source": "note"}}"#
        ));
        contents.push('\n');
    }
    // A handful of malformed lines mixed in should be skipped, not panic.
    contents.push_str("not json at all\n");
    contents.push_str(r#"{"wrong_field": true}"#);
    contents.push('\n');
    std::fs::write(&path, contents).expect("write memory file");

    let source = FileMemorySource::new(path);
    let messages = vec![Message::user().with_text("what do we know about databases?")];
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        source.retrieve(&MemoryQuery {
            session_id: "test",
            messages: &messages,
            reserved_tokens: 1_000,
        })
    }));
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "must not panic on a 20K-line memory file");
    let items = result.unwrap();
    assert!(
        items.len() <= 16,
        "recall must stay bounded by MAX_RECALLED_ITEMS regardless of file size"
    );
    assert!(
        elapsed.as_secs() < 5,
        "recall over a 20K-line file took too long: {:?}",
        elapsed
    );
}
