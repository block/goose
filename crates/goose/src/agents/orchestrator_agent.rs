//! OrchestratorAgent — LLM-based meta-coordinator for multi-agent routing.
//!
//! Replaces the keyword-based IntentRouter with an LLM that understands context,
//! domain, and request complexity. Falls back to IntentRouter when LLM is unavailable.
//!
//! # Architecture
//!
//! ```text
//! User Message → OrchestratorAgent.route()
//!   ├─ Build agent catalog from GooseAgent + DeveloperAgent + external agents
//!   ├─ Render routing prompt with catalog + user message
//!   ├─ LLM classifies intent → RoutingDecision (single or compound)
//!   ├─ (fallback) IntentRouter keyword matching
//!   └─ Return OrchestratorPlan with one or more sub-tasks
//! ```
//!
//! # Compound Request Splitting
//!
//! When a user message contains multiple independent intents (e.g., "fix the login
//! bug and add a dark theme"), the orchestrator splits it into sub-tasks, each
//! routed to the appropriate agent/mode. Results are aggregated into a coherent
//! response.
//!
//! # Feature Flag
//!
//! LLM routing + splitting is enabled by default.
//! Set `GOOSE_ORCHESTRATOR_DISABLED=true` to fall back to keyword routing.
//! When disabled (default), falls back to IntentRouter for backward compatibility.

use crate::agents::developer_agent::DeveloperAgent;
use crate::agents::goose_agent::GooseAgent;
use crate::agents::intent_router::{IntentRouter, RoutingDecision};
use crate::agents::pm_agent::PmAgent;
use crate::agents::qa_agent::QaAgent;
use crate::agents::research_agent::ResearchAgent;
use crate::agents::security_agent::SecurityAgent;
use crate::context_mgmt::{
    check_if_compaction_needed, compact_messages, DEFAULT_COMPACTION_THRESHOLD,
};
use crate::conversation::Conversation;
use crate::prompt_template;
use crate::providers::base::{Provider, ProviderUsage};
use crate::session::Session;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, warn, Span};

/// Thread-safe flag for LLM-based orchestration.
/// Initialized from GOOSE_ORCHESTRATOR_DISABLED env var on first access,
/// then controllable via set_orchestrator_enabled() without unsafe env mutation.
static ORCHESTRATOR_DISABLED: AtomicBool = AtomicBool::new(false);
static ORCHESTRATOR_INIT: std::sync::Once = std::sync::Once::new();

fn init_orchestrator_flag() {
    ORCHESTRATOR_INIT.call_once(|| {
        let disabled = std::env::var("GOOSE_ORCHESTRATOR_DISABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        ORCHESTRATOR_DISABLED.store(disabled, Ordering::Relaxed);
    });
}

/// Whether LLM-based orchestration is enabled.
/// Reads the env var once at startup, then uses a thread-safe atomic flag.
pub fn is_orchestrator_enabled() -> bool {
    init_orchestrator_flag();
    !ORCHESTRATOR_DISABLED.load(Ordering::Relaxed)
}

/// Disable LLM-based orchestration (thread-safe, no env mutation).
pub fn set_orchestrator_enabled(enabled: bool) {
    init_orchestrator_flag();
    ORCHESTRATOR_DISABLED.store(!enabled, Ordering::Relaxed);
}

/// Context for rendering the orchestrator routing prompt.
#[derive(Serialize)]
struct RoutingPromptContext {
    user_message: String,
    agent_catalog: String,
}

/// A sub-task produced by compound request splitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub task_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub routing: RoutingDecision,
    pub sub_task_description: String,
}

/// The plan produced by the orchestrator for a user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorPlan {
    pub is_compound: bool,
    pub tasks: Vec<SubTask>,
}

impl OrchestratorPlan {
    /// Create a simple plan with a single routing decision (no splitting).
    pub fn single(decision: RoutingDecision) -> Self {
        let desc = decision.reasoning.clone();
        Self {
            is_compound: false,
            tasks: vec![SubTask {
                task_id: "task-1".to_string(),
                depends_on: Vec::new(),
                routing: decision,
                sub_task_description: desc,
            }],
        }
    }

    /// Get the primary routing decision (first task).
    pub fn primary_routing(&self) -> &RoutingDecision {
        &self.tasks[0].routing
    }
}

/// A structured plan proposal returned by plan() — ready for client display and confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposal {
    pub is_compound: bool,
    pub tasks: Vec<PlanProposalTask>,
    pub clarifying_questions: Option<Vec<String>>,
}

/// A single task within a plan proposal, enriched with display info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposalTask {
    pub task_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub agent_name: String,
    pub mode_slug: String,
    pub mode_name: String,
    pub confidence: f32,
    pub reasoning: String,
    pub description: String,
    pub tool_groups: Vec<String>,
}

/// The OrchestratorAgent coordinates routing decisions using LLM intelligence.
///
/// It maintains a dynamic catalog of agents/modes via the internal IntentRouter.
/// That catalog is rendered into the LLM prompt so it can make informed routing
/// decisions.
///
/// The orchestrator should *not* hardcode mode slugs in its prompt. Instead, it
/// relies on the agent catalog and each mode's `when_to_use` guidance.
pub struct OrchestratorAgent {
    intent_router: IntentRouter,
    provider: Arc<Mutex<Option<Arc<dyn Provider>>>>,
}

impl OrchestratorAgent {
    pub fn new(provider: Arc<Mutex<Option<Arc<dyn Provider>>>>) -> Self {
        Self {
            intent_router: IntentRouter::new(),
            provider,
        }
    }

    fn slot_for(&self, agent_name: &str) -> Option<&crate::agents::intent_router::AgentSlot> {
        self.intent_router
            .slots()
            .iter()
            .find(|s| s.name == agent_name)
    }

    /// Expose the inner IntentRouter for state synchronization (enable/disable, extensions).
    pub fn intent_router_mut(&mut self) -> &mut IntentRouter {
        &mut self.intent_router
    }

    /// Set enabled state for an agent slot (delegates to IntentRouter).
    pub fn set_enabled(&mut self, agent_name: &str, enabled: bool) {
        self.intent_router.set_enabled(agent_name, enabled);
    }

    /// Set bound extensions for an agent slot (delegates to IntentRouter).
    pub fn set_bound_extensions(&mut self, agent_name: &str, extensions: Vec<String>) {
        self.intent_router
            .set_bound_extensions(agent_name, extensions);
    }

    /// Get the agent slots (delegates to IntentRouter).
    pub fn slots(&self) -> &[crate::agents::intent_router::AgentSlot] {
        self.intent_router.slots()
    }

    /// Route a user message to the best agent and mode, with optional compound splitting.
    ///
    /// Returns an `OrchestratorPlan` that may contain multiple sub-tasks for
    /// compound requests when LLM orchestration is enabled.
    #[instrument(
        name = "orchestrator.route",
        skip(self, user_message),
        fields(
            otel.kind = "internal",
            orchestrator.llm_enabled = is_orchestrator_enabled(),
            orchestrator.strategy,
            orchestrator.is_compound,
            orchestrator.task_count,
            orchestrator.primary_agent,
            orchestrator.primary_mode,
            orchestrator.primary_confidence,
        )
    )]
    pub async fn route(&self, user_message: &str) -> OrchestratorPlan {
        let span = Span::current();

        if is_orchestrator_enabled() {
            match self.route_with_llm(user_message).await {
                Ok(plan) => {
                    let primary = plan.primary_routing();
                    span.record("orchestrator.strategy", "llm");
                    span.record("orchestrator.is_compound", plan.is_compound);
                    span.record("orchestrator.task_count", plan.tasks.len() as i64);
                    span.record("orchestrator.primary_agent", primary.agent_name.as_str());
                    span.record("orchestrator.primary_mode", primary.mode_slug.as_str());
                    span.record("orchestrator.primary_confidence", primary.confidence as f64);

                    info!(
                        is_compound = plan.is_compound,
                        task_count = plan.tasks.len(),
                        primary_agent = %primary.agent_name,
                        primary_mode = %primary.mode_slug,
                        primary_confidence = %primary.confidence,
                        "LLM orchestrator routed message"
                    );

                    for (i, task) in plan.tasks.iter().enumerate() {
                        info!(
                            task_index = i,
                            agent = %task.routing.agent_name,
                            mode = %task.routing.mode_slug,
                            confidence = %task.routing.confidence,
                            description = %task.sub_task_description,
                            "Orchestrator sub-task"
                        );
                    }

                    return plan;
                }
                Err(e) => {
                    span.record("orchestrator.strategy", "keyword_fallback");
                    warn!(error = %e, "LLM routing failed, falling back to keyword matching");
                }
            }
        } else {
            span.record("orchestrator.strategy", "keyword");
        }

        // Fallback to keyword-based IntentRouter (always single-task)
        let decision = self.intent_router.route(user_message);
        span.record("orchestrator.is_compound", false);
        span.record("orchestrator.task_count", 1i64);
        span.record("orchestrator.primary_agent", decision.agent_name.as_str());
        span.record("orchestrator.primary_mode", decision.mode_slug.as_str());
        span.record(
            "orchestrator.primary_confidence",
            decision.confidence as f64,
        );

        debug!(
            agent_name = %decision.agent_name,
            mode_slug = %decision.mode_slug,
            confidence = %decision.confidence,
            "Keyword router decision"
        );
        OrchestratorPlan::single(decision)
    }

    /// Produce a structured plan without executing — for client display and confirmation.
    ///
    /// Uses the same routing/splitting logic as route(), but enriches the result
    /// with mode descriptions and tool groups for human-readable display.
    /// The client can then confirm the plan and send it back via execute_plan.
    pub async fn plan(&self, user_message: &str) -> PlanProposal {
        let orch_plan = self.route(user_message).await;

        let tasks: Vec<PlanProposalTask> = orch_plan
            .tasks
            .iter()
            .map(|sub_task| {
                let mode_name =
                    self.get_mode_name(&sub_task.routing.agent_name, &sub_task.routing.mode_slug);
                let tool_groups = self
                    .get_tool_groups_for_routing(
                        &sub_task.routing.agent_name,
                        &sub_task.routing.mode_slug,
                    )
                    .iter()
                    .map(|tg| match tg {
                        crate::registry::manifest::ToolGroupAccess::Full(name) => name.clone(),
                        crate::registry::manifest::ToolGroupAccess::Restricted {
                            group, ..
                        } => group.clone(),
                    })
                    .collect();

                PlanProposalTask {
                    task_id: sub_task.task_id.clone(),
                    depends_on: sub_task.depends_on.clone(),
                    agent_name: sub_task.routing.agent_name.clone(),
                    mode_slug: sub_task.routing.mode_slug.clone(),
                    mode_name,
                    confidence: sub_task.routing.confidence,
                    reasoning: sub_task.routing.reasoning.clone(),
                    description: sub_task.sub_task_description.clone(),
                    tool_groups,
                }
            })
            .collect();

        info!(
            is_compound = orch_plan.is_compound,
            task_count = tasks.len(),
            "Plan proposal generated"
        );

        PlanProposal {
            is_compound: orch_plan.is_compound,
            tasks,
            clarifying_questions: None,
        }
    }
    /// Look up a human-readable mode name from the catalog.
    fn get_mode_name(&self, agent_name: &str, mode_slug: &str) -> String {
        let slot = match self.slot_for(agent_name) {
            Some(s) => s,
            None => return mode_slug.to_string(),
        };

        slot.modes
            .iter()
            .find(|m| m.slug == mode_slug)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| mode_slug.to_string())
    }

    /// Use the LLM to classify the user's intent, potentially splitting compound requests.
    #[instrument(
        name = "orchestrator.llm_classify",
        skip(self),
        fields(
            orchestrator.catalog_agents = self.intent_router.slots().len() as i64,
            orchestrator.llm_response_parsed,
        )
    )]
    async fn route_with_llm(&self, user_message: &str) -> Result<OrchestratorPlan> {
        let provider_guard = self.provider.lock().await;
        let provider = provider_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No provider available for LLM routing"))?;

        let catalog_text = self.build_catalog_text();
        let context = RoutingPromptContext {
            user_message: user_message.to_string(),
            agent_catalog: catalog_text,
        };

        let splitting_prompt =
            prompt_template::render_template("orchestrator/splitting.md", &context)?;

        let messages = vec![crate::conversation::message::Message::user().with_text(user_message)];

        let (response, _usage) = provider
            .complete("orchestrator-routing", &splitting_prompt, &messages, &[])
            .await?;

        self.parse_splitting_response(&response)
    }

    /// Build a human-readable catalog of all available agents and their modes.
    pub fn build_catalog_text(&self) -> String {
        let mut text = String::new();

        for slot in self.intent_router.slots().iter().filter(|s| s.enabled) {
            text.push_str(&format!(
                "### {} \u{2014} {}\n",
                slot.name, slot.description
            ));
            text.push_str(&format!("Default mode: {}\n", slot.default_mode));
            text.push_str("Modes:\n");
            for mode in &slot.modes {
                let when = mode.when_to_use.as_deref().unwrap_or(&mode.description);
                text.push_str(&format!(
                    "  - **{}** ({}): {} | Use when: {}\n",
                    mode.slug, mode.name, mode.description, when
                ));
            }
            text.push('\n');
        }
        text
    }

    /// Parse the LLM's splitting response into an OrchestratorPlan.
    fn parse_splitting_response(
        &self,
        response: &crate::conversation::message::Message,
    ) -> Result<OrchestratorPlan> {
        let text = response
            .content
            .iter()
            .filter_map(|c| match c {
                crate::conversation::message::MessageContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let json_str = extract_json(&text)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let is_compound = parsed["is_compound"].as_bool().unwrap_or(false);
        let tasks_arr = parsed["tasks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in splitting response"))?;

        if tasks_arr.is_empty() {
            return Err(anyhow::anyhow!("Empty tasks array in splitting response"));
        }

        let mut tasks = Vec::new();
        let mut next_generated_id: usize = 1;
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for task_val in tasks_arr {
            let mut task_id = task_val["task_id"]
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    let id = format!("task-{}", next_generated_id);
                    next_generated_id += 1;
                    id
                });

            if seen_ids.contains(&task_id) {
                let mut suffix = 2usize;
                loop {
                    let candidate = format!("{}-{}", task_id, suffix);
                    if !seen_ids.contains(&candidate) {
                        task_id = candidate;
                        break;
                    }
                    suffix += 1;
                }
            }
            seen_ids.insert(task_id.clone());

            let depends_on = task_val["depends_on"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let agent_name = task_val["agent_name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing agent_name in task"))?;
            let mode_slug = task_val["mode_slug"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing mode_slug in task"))?;
            let confidence = task_val["confidence"].as_f64().unwrap_or(0.5) as f32;
            let reasoning = task_val["reasoning"]
                .as_str()
                .unwrap_or("LLM routing decision")
                .to_string();
            let sub_task = task_val["sub_task"]
                .as_str()
                .unwrap_or(agent_name)
                .to_string();

            let slot = match self.slot_for(agent_name) {
                Some(s) => s,
                None => {
                    warn!(
                        "LLM selected unknown agent '{}', skipping sub-task",
                        agent_name
                    );
                    continue;
                }
            };

            // Validate mode_slug
            let resolved_mode_slug = if slot.modes.iter().any(|m| m.slug == mode_slug) {
                mode_slug.to_string()
            } else {
                warn!(
                    agent = agent_name,
                    mode_slug = mode_slug,
                    default_mode = slot.default_mode,
                    "LLM selected unknown mode for agent; falling back to default"
                );
                slot.default_mode.clone()
            };

            tasks.push(SubTask {
                task_id,
                depends_on,
                routing: RoutingDecision {
                    agent_name: agent_name.to_string(),
                    mode_slug: resolved_mode_slug,
                    confidence,
                    reasoning,
                },
                sub_task_description: sub_task,
            });
        }

        if tasks.is_empty() {
            return Err(anyhow::anyhow!(
                "No valid tasks after filtering, all agent names were unknown"
            ));
        }

        // Normalize dependencies: keep only references to known task IDs and avoid self-deps.
        // Use owned Strings to avoid borrowing `tasks` across the mutation loop.
        let known: std::collections::HashSet<String> =
            tasks.iter().map(|t| t.task_id.clone()).collect();
        for task in &mut tasks {
            let self_id = task.task_id.clone();
            task.depends_on
                .retain(|dep| dep != &self_id && known.contains(dep));
        }

        // Deterministic ordering: topological if possible, otherwise stable by task_id.
        if let Some(order) = topo_sort_tasks(&tasks) {
            tasks = order;
        } else {
            tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        }

        Ok(OrchestratorPlan { is_compound, tasks })
    }

    /// Check if the conversation needs compaction before delegating to a sub-agent.
    ///
    /// The orchestrator is the right place for this check because it has visibility
    /// across all agents and can compact proactively before routing, rather than
    /// waiting for an agent to hit its context limit mid-reply.
    pub async fn check_compaction_needed(
        &self,
        conversation: &Conversation,
        session: &Session,
    ) -> Result<bool> {
        let provider_guard = self.provider.lock().await;
        let provider = match provider_guard.as_ref() {
            Some(p) => p,
            None => return Ok(false),
        };
        check_if_compaction_needed(provider.as_ref(), conversation, None, session).await
    }

    /// Perform proactive compaction if the conversation exceeds the threshold.
    ///
    /// Returns the compacted conversation and usage info if compaction was performed,
    /// or None if compaction wasn't needed.
    pub async fn compact_if_needed(
        &self,
        session_id: &str,
        conversation: &Conversation,
        session: &Session,
    ) -> Result<Option<(Conversation, ProviderUsage)>> {
        if !self.check_compaction_needed(conversation, session).await? {
            return Ok(None);
        }

        let provider_guard = self.provider.lock().await;
        let provider = provider_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No provider available for compaction"))?;

        let config = crate::config::Config::global();
        let threshold = config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
        let threshold_pct = (threshold * 100.0) as u32;

        info!(
            threshold = threshold_pct,
            "Orchestrator: proactive compaction triggered"
        );

        let result = compact_messages(provider.as_ref(), session_id, conversation, false).await?;
        Ok(Some(result))
    }

    /// Get the tool_groups for a given routing decision.
    ///
    /// Looks up the mode's tool_groups from GooseAgent or DeveloperAgent
    /// based on the routing decision's agent_name and mode_slug.
    /// Returns empty Vec if the mode isn't found (which means "all tools" — backward compatible).
    pub fn get_tool_groups_for_routing(
        &self,
        agent_name: &str,
        mode_slug: &str,
    ) -> Vec<crate::registry::manifest::ToolGroupAccess> {
        let slot = match self.slot_for(agent_name) {
            Some(s) => s,
            None => return vec![],
        };

        let mode = slot
            .modes
            .iter()
            .find(|m| m.slug == mode_slug)
            .or_else(|| slot.modes.iter().find(|m| m.slug == slot.default_mode));

        mode.map(|m| m.tool_groups.clone()).unwrap_or_default()
    }

    /// Get the recommended MCP extensions for a specific agent/mode.
    /// Used by reply.rs to activate only the extensions needed by the current mode.
    pub fn get_recommended_extensions_for_routing(
        &self,
        agent_name: &str,
        mode_slug: &str,
    ) -> Vec<String> {
        match agent_name {
            "Goose Agent" => {
                let goose = GooseAgent::new();
                if let Some(mode) = goose.mode(mode_slug) {
                    mode.recommended_extensions.clone()
                } else {
                    vec![]
                }
            }
            "Developer Agent" => {
                let dev = DeveloperAgent::new();
                dev.recommended_extensions(mode_slug)
            }
            "QA Agent" => {
                let qa = QaAgent::new();
                qa.recommended_extensions(mode_slug)
            }
            "PM Agent" => {
                let pm = PmAgent::new();
                pm.recommended_extensions(mode_slug)
            }
            "Security Agent" => {
                let security = SecurityAgent::new();
                security.recommended_extensions(mode_slug)
            }
            "Research Agent" => {
                let research = ResearchAgent::new();
                research.recommended_extensions(mode_slug)
            }
            _ => vec![], // external agent \u2192 no restrictions
        }
    }

    /// Apply the routing decision to an agent: set tool groups, allowed extensions,
    /// and orchestrator context based on the primary routing target.
    ///
    /// This centralizes the "mode is dispatch" pattern so that all entrypoints
    /// (reply, runs, ACP-IDE) apply routing consistently.
    pub async fn apply_routing(
        &self,
        agent: &crate::agents::agent::Agent,
        plan: &OrchestratorPlan,
    ) {
        let primary = plan.primary_routing();

        let mut allowed_extensions: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();

        // Apply bound extensions from the agent slot
        if let Some(slot) = self.slot_for(&primary.agent_name) {
            allowed_extensions.extend(slot.bound_extensions.iter().cloned());
        }

        // Set orchestrator context flag
        agent
            .set_orchestrator_context(is_orchestrator_enabled())
            .await;

        // Persist the active mode slug so downstream systems (like genui validation)
        // can apply mode-specific behavior.
        agent
            .set_active_mode_slug(Some(primary.mode_slug.clone()))
            .await;

        // Apply mode-specific tool groups
        let tool_groups = self.get_tool_groups_for_routing(&primary.agent_name, &primary.mode_slug);
        if !tool_groups.is_empty() {
            agent.set_active_tool_groups(tool_groups).await;
        }

        // Apply mode-recommended extensions
        let recommended =
            self.get_recommended_extensions_for_routing(&primary.agent_name, &primary.mode_slug);
        allowed_extensions.extend(recommended);

        if !allowed_extensions.is_empty() {
            agent
                .set_allowed_extensions(allowed_extensions.into_iter().collect())
                .await;
        }

        tracing::info!(
            agent = %primary.agent_name,
            mode = %primary.mode_slug,
            confidence = %primary.confidence,
            "Applied routing bindings to agent"
        );
    }
}

/// Aggregate results from multiple sub-tasks into a coherent response.
///
/// Takes the sub-task descriptions and their results, and produces a
/// combined message that presents all results clearly.
pub fn aggregate_results(tasks: &[SubTask], results: &[String]) -> String {
    if tasks.len() == 1 {
        return results.first().cloned().unwrap_or_default();
    }

    let mut output = String::from("I handled your compound request in multiple parts:\n\n");
    for (i, (task, result)) in tasks.iter().zip(results.iter()).enumerate() {
        output.push_str(&format!(
            "## Part {} — {}\n\n{}\n\n",
            i + 1,
            task.sub_task_description,
            result
        ));
    }
    output
}

fn topo_sort_tasks(tasks: &[SubTask]) -> Option<Vec<SubTask>> {
    let order = topo_sort_indices(tasks)?;
    let mut with_index: Vec<(usize, &SubTask)> = tasks.iter().enumerate().collect();
    with_index.sort_by_key(|(idx, _)| order.get(idx).copied().unwrap_or(usize::MAX));
    Some(with_index.into_iter().map(|(_, t)| t.clone()).collect())
}

fn topo_sort_indices(tasks: &[SubTask]) -> Option<std::collections::HashMap<usize, usize>> {
    use std::collections::{BTreeSet, HashMap};

    let mut id_to_index: HashMap<&str, usize> = HashMap::new();
    for (idx, task) in tasks.iter().enumerate() {
        id_to_index.insert(task.task_id.as_str(), idx);
    }

    let mut in_degree = vec![0usize; tasks.len()];
    let mut out_edges: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];

    for (idx, task) in tasks.iter().enumerate() {
        for dep in &task.depends_on {
            if let Some(&dep_idx) = id_to_index.get(dep.as_str()) {
                in_degree[idx] += 1;
                out_edges[dep_idx].push(idx);
            }
        }
    }

    // Deterministic tie-breaker: smallest task_id among ready nodes.
    let mut ready: BTreeSet<(String, usize)> = BTreeSet::new();
    for (idx, deg) in in_degree.iter().enumerate() {
        if *deg == 0 {
            ready.insert((tasks[idx].task_id.clone(), idx));
        }
    }

    let mut order_map: HashMap<usize, usize> = HashMap::new();
    let mut visited = 0usize;
    let mut next = 0usize;

    while let Some((_, node)) = ready.pop_first() {
        order_map.insert(node, next);
        next += 1;
        visited += 1;

        for &child in &out_edges[node] {
            in_degree[child] = in_degree[child].saturating_sub(1);
            if in_degree[child] == 0 {
                ready.insert((tasks[child].task_id.clone(), child));
            }
        }
    }

    if visited == tasks.len() {
        Some(order_map)
    } else {
        None
    }
}

/// Extract a JSON object from text that may contain markdown code fences.
fn extract_json(text: &str) -> Result<String> {
    let fence = "```";
    let fence_json = "```json";

    // Try to find JSON in code blocks first
    if let Some(start) = text.find(fence_json) {
        if let Some(after_fence) = text.get(start + fence_json.len()..) {
            if let Some(end) = after_fence.find(fence) {
                if let Some(content) = after_fence.get(..end) {
                    return Ok(content.trim().to_string());
                }
            }
        }
    }
    if let Some(start) = text.find(fence) {
        if let Some(after_fence) = text.get(start + fence.len()..) {
            if let Some(end) = after_fence.find(fence) {
                if let Some(content) = after_fence.get(..end) {
                    let inner = content.trim();
                    if inner.starts_with('{') {
                        return Ok(inner.to_string());
                    }
                }
            }
        }
    }

    // Try to find raw JSON object
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Some(content) = text.get(start..=end) {
                return Ok(content.to_string());
            }
        }
    }

    Err(anyhow::anyhow!("No JSON object found in LLM response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_orchestrator() -> OrchestratorAgent {
        OrchestratorAgent::new(Arc::new(Mutex::new(None)))
    }

    #[test]
    fn test_build_catalog_text() {
        let orch = make_orchestrator();
        let catalog = orch.build_catalog_text();

        assert!(catalog.contains("Goose Agent"));
        assert!(catalog.contains("Developer Agent"));
        assert!(catalog.contains("ask"));
        assert!(catalog.contains("write"));
    }

    #[test]
    fn test_parse_single_task_response() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": false, "tasks": [{"agent_name": "Developer Agent", "mode_slug": "write", "confidence": 0.9, "reasoning": "API implementation task", "sub_task": "implement a REST API endpoint"}]}"#,
        );

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert!(!plan.is_compound);
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.primary_routing().agent_name, "Developer Agent");
        assert_eq!(plan.primary_routing().mode_slug, "write");
        assert_eq!(
            plan.tasks[0].sub_task_description,
            "implement a REST API endpoint"
        );
    }

    #[test]
    fn test_parse_compound_response() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": true, "tasks": [
                {"agent_name": "Developer Agent", "mode_slug": "write", "confidence": 0.85, "reasoning": "Bug fix", "sub_task": "Fix the login endpoint bug"},
                {"agent_name": "Developer Agent", "mode_slug": "write", "confidence": 0.8, "reasoning": "UI feature", "sub_task": "Add dark theme toggle to settings"}
            ]}"#,
        );

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert!(plan.is_compound);
        assert_eq!(plan.tasks.len(), 2);
        assert_eq!(plan.tasks[0].routing.agent_name, "Developer Agent");
        assert_eq!(plan.tasks[0].routing.mode_slug, "write");
        assert_eq!(
            plan.tasks[0].sub_task_description,
            "Fix the login endpoint bug"
        );
        assert_eq!(plan.tasks[1].routing.mode_slug, "write");
        assert_eq!(
            plan.tasks[1].sub_task_description,
            "Add dark theme toggle to settings"
        );
    }

    #[test]
    fn test_parse_compound_response_with_dependencies() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": true, "tasks": [
                {"task_id": "a", "agent_name": "Developer Agent", "mode_slug": "write", "confidence": 0.85, "reasoning": "Bug fix", "sub_task": "Fix the login endpoint bug"},
                {"task_id": "b", "depends_on": ["a"], "agent_name": "Developer Agent", "mode_slug": "write", "confidence": 0.8, "reasoning": "UI feature", "sub_task": "Add dark theme toggle to settings"}
            ]}"#,
        );

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert!(plan.is_compound);
        assert_eq!(plan.tasks.len(), 2);

        assert_eq!(plan.tasks[0].task_id, "a");
        assert_eq!(plan.tasks[0].depends_on, Vec::<String>::new());

        assert_eq!(plan.tasks[1].task_id, "b");
        assert_eq!(plan.tasks[1].depends_on, vec!["a".to_string()]);

        // Topological ordering should ensure "a" runs before "b".
        assert_eq!(plan.tasks[0].task_id, "a");
        assert_eq!(plan.tasks[1].task_id, "b");
    }

    #[test]
    fn test_parse_compound_response_dependency_normalization() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": true, "tasks": [
                {"task_id": "a", "depends_on": ["a", "missing"], "agent_name": "Goose Agent", "mode_slug": "ask", "confidence": 0.9, "reasoning": "A", "sub_task": "A"},
                {"task_id": "b", "depends_on": ["a"], "agent_name": "Goose Agent", "mode_slug": "ask", "confidence": 0.9, "reasoning": "B", "sub_task": "B"}
            ]}"#,
        );

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert_eq!(plan.tasks.len(), 2);

        let a = plan.tasks.iter().find(|t| t.task_id == "a").unwrap();
        assert!(
            a.depends_on.is_empty(),
            "self-dep and unknown dep should be removed"
        );
    }

    #[test]
    fn test_parse_response_markdown_wrapped() {
        let orch = make_orchestrator();

        let text = concat!(
            "Here's my analysis:\n\n",
            "```json\n",
            r#"{"is_compound": false, "tasks": [{"agent_name": "Goose Agent", "mode_slug": "planner", "confidence": 0.85, "reasoning": "Planning task", "sub_task": "Create a project plan"}]}"#,
            "\n```"
        );
        let response = crate::conversation::message::Message::assistant().with_text(text);

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert!(!plan.is_compound);
        assert_eq!(plan.primary_routing().agent_name, "Goose Agent");
        assert_eq!(plan.primary_routing().mode_slug, "planner");
    }

    #[test]
    fn test_parse_response_invalid_agent_filtered() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": true, "tasks": [
                {"agent_name": "NonExistent Agent", "mode_slug": "foo", "confidence": 0.5, "reasoning": "test", "sub_task": "invalid"},
                {"agent_name": "Goose Agent", "mode_slug": "ask", "confidence": 0.8, "reasoning": "fallback", "sub_task": "valid task"}
            ]}"#,
        );

        let plan = orch.parse_splitting_response(&response).unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].routing.agent_name, "Goose Agent");
    }

    #[test]
    fn test_parse_response_all_invalid_agents() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant().with_text(
            r#"{"is_compound": false, "tasks": [{"agent_name": "NonExistent", "mode_slug": "x", "confidence": 0.5, "reasoning": "t", "sub_task": "y"}]}"#,
        );

        assert!(orch.parse_splitting_response(&response).is_err());
    }

    #[test]
    fn test_parse_response_empty_tasks() {
        let orch = make_orchestrator();

        let response = crate::conversation::message::Message::assistant()
            .with_text(r#"{"is_compound": false, "tasks": []}"#);

        assert!(orch.parse_splitting_response(&response).is_err());
    }

    #[test]
    fn test_orchestrator_plan_single() {
        let decision = RoutingDecision {
            agent_name: "Goose Agent".into(),
            mode_slug: "ask".into(),
            confidence: 0.9,
            reasoning: "General question".into(),
        };
        let plan = OrchestratorPlan::single(decision);

        assert!(!plan.is_compound);
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.primary_routing().agent_name, "Goose Agent");
    }

    #[test]
    fn test_aggregate_results_single() {
        let tasks = vec![SubTask {
            task_id: "task-1".into(),
            depends_on: Vec::new(),
            routing: RoutingDecision {
                agent_name: "Goose Agent".into(),
                mode_slug: "ask".into(),
                confidence: 0.9,
                reasoning: "test".into(),
            },
            sub_task_description: "Answer the question".into(),
        }];
        let results = vec!["The answer is 42.".into()];

        let output = aggregate_results(&tasks, &results);
        assert_eq!(output, "The answer is 42.");
    }

    #[test]
    fn test_aggregate_results_compound() {
        let tasks = vec![
            SubTask {
                task_id: "task-1".into(),
                depends_on: Vec::new(),
                routing: RoutingDecision {
                    agent_name: "Developer Agent".into(),
                    mode_slug: "code".into(),
                    confidence: 0.8,
                    reasoning: "bug fix".into(),
                },
                sub_task_description: "Fix login bug".into(),
            },
            SubTask {
                task_id: "task-2".into(),
                depends_on: vec!["task-1".into()],
                routing: RoutingDecision {
                    agent_name: "Developer Agent".into(),
                    mode_slug: "frontend".into(),
                    confidence: 0.8,
                    reasoning: "UI feature".into(),
                },
                sub_task_description: "Add dark theme".into(),
            },
        ];
        let results = vec!["Login bug fixed.".into(), "Dark theme added.".into()];

        let output = aggregate_results(&tasks, &results);
        assert!(output.contains("Part 1"));
        assert!(output.contains("Fix login bug"));
        assert!(output.contains("Login bug fixed."));
        assert!(output.contains("Part 2"));
        assert!(output.contains("Add dark theme"));
        assert!(output.contains("Dark theme added."));
    }

    #[test]
    fn test_extract_json_raw() {
        let text = r#"{"is_compound": false, "tasks": [{"agent_name": "Goose Agent"}]}"#;
        let json = extract_json(text).unwrap();
        assert!(json.contains("Goose Agent"));
    }

    #[test]
    fn test_extract_json_code_block() {
        let text = concat!(
            "Some text\n",
            "```json\n",
            r#"{"key": "value"}"#,
            "\n```\n",
            "More text"
        );
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_no_json() {
        let text = "Just plain text with no JSON";
        assert!(extract_json(text).is_err());
    }

    #[tokio::test]
    async fn test_route_fallback_to_keyword() {
        let orch = make_orchestrator();

        let plan = orch
            .route("implement a REST API endpoint for user authentication")
            .await;

        assert!(!plan.is_compound);
        assert_eq!(plan.tasks.len(), 1);
        assert!(!plan.primary_routing().agent_name.is_empty());
        assert!(!plan.primary_routing().mode_slug.is_empty());
    }

    #[test]
    fn test_orchestrator_can_be_disabled() {
        // Use the thread-safe API instead of mutating env vars
        set_orchestrator_enabled(false);
        assert!(!is_orchestrator_enabled());
        // Restore default state
        set_orchestrator_enabled(true);
        assert!(is_orchestrator_enabled());
    }

    #[test]
    fn test_catalog_excludes_compactor_mode() {
        let orch = make_orchestrator();
        let catalog = orch.build_catalog_text();
        // Compactor should not appear as a routable mode since it's
        // an orchestrator-level concern, not a user-facing agent mode
        assert!(
            !catalog.contains("compactor"),
            "Compactor mode should be excluded from the routing catalog"
        );
    }

    #[test]
    fn test_orchestrator_has_compaction_methods() {
        let orch = make_orchestrator();
        // Verify the orchestrator exposes compaction coordination methods.
        // Actual async compaction tests require a real provider + session,
        // so we verify the API surface exists and the struct is well-formed.
        assert!(orch.provider.try_lock().is_ok());
    }

    #[tokio::test]
    async fn test_plan_produces_proposal() {
        let orch = make_orchestrator();

        let proposal = orch
            .plan("implement a REST API endpoint for user authentication")
            .await;

        assert!(!proposal.tasks.is_empty());
        let task = &proposal.tasks[0];
        assert!(!task.agent_name.is_empty());
        assert!(!task.mode_slug.is_empty());
        assert!(!task.mode_name.is_empty());
        assert!(!task.description.is_empty());
        assert!(proposal.clarifying_questions.is_none());
    }

    #[test]
    fn test_plan_proposal_serialization() {
        let proposal = PlanProposal {
            is_compound: true,
            tasks: vec![
                PlanProposalTask {
                    task_id: "task-1".into(),
                    depends_on: Vec::new(),
                    agent_name: "Developer Agent".into(),
                    mode_slug: "write".into(),
                    mode_name: "✏️ Write".into(),
                    confidence: 0.85,
                    reasoning: "API implementation".into(),
                    description: "Build the REST endpoint".into(),
                    tool_groups: vec![
                        "developer".into(),
                        "read".into(),
                        "edit".into(),
                        "command".into(),
                        "fetch".into(),
                        "memory".into(),
                    ],
                },
                PlanProposalTask {
                    task_id: "task-2".into(),
                    depends_on: vec!["task-1".into()],
                    agent_name: "QA Agent".into(),
                    mode_slug: "review".into(),
                    mode_name: "🔍 Review".into(),
                    confidence: 0.75,
                    reasoning: "Testing needed".into(),
                    description: "Write integration tests".into(),
                    tool_groups: vec!["read".into(), "memory".into()],
                },
            ],
            clarifying_questions: None,
        };

        let json = serde_json::to_string(&proposal).unwrap();
        let deserialized: PlanProposal = serde_json::from_str(&json).unwrap();

        assert!(deserialized.is_compound);
        assert_eq!(deserialized.tasks.len(), 2);
        assert_eq!(deserialized.tasks[0].agent_name, "Developer Agent");
        assert_eq!(deserialized.tasks[0].mode_slug, "write");
        assert_eq!(deserialized.tasks[0].mode_name, "✏️ Write");
        assert_eq!(deserialized.tasks[1].mode_slug, "review");
    }

    #[test]
    fn test_plan_proposal_with_clarifying_questions() {
        let proposal = PlanProposal {
            is_compound: false,
            tasks: vec![],
            clarifying_questions: Some(vec![
                "What database should I use?".into(),
                "Should the API support pagination?".into(),
            ]),
        };

        let json = serde_json::to_string(&proposal).unwrap();
        let deserialized: PlanProposal = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.is_compound);
        assert!(deserialized.tasks.is_empty());
        let questions = deserialized.clarifying_questions.unwrap();
        assert_eq!(questions.len(), 2);
        assert!(questions[0].contains("database"));
    }

    #[test]
    fn test_get_mode_name_found() {
        let orch = make_orchestrator();
        let name = orch.get_mode_name("Goose Agent", "ask");
        assert!(
            name.contains("Ask"),
            "Expected mode name containing 'Ask', got: {}",
            name
        );
    }

    #[test]
    fn test_get_mode_name_fallback() {
        let orch = make_orchestrator();
        let name = orch.get_mode_name("NonExistent", "unknown");
        assert_eq!(name, "unknown");
    }
}
