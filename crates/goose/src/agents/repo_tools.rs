use indoc::indoc;
use once_cell::sync::Lazy;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};

#[cfg(feature = "repo-index")]
use crate::repo_index::{RepoIndexOptions};
#[cfg(feature = "repo-index")]
use crate::repo_index::service::RepoIndexService;
#[cfg(feature = "repo-index")]
use anyhow::{anyhow, Result};

// Tool name constants
pub const REPO_QUERY_TOOL_NAME: &str = "repo__search";
pub const REPO_STATS_TOOL_NAME: &str = "repo__stats";

#[derive(Clone)]
struct CachedIndex {
    service: Arc<RepoIndexService>,
    built_at: std::time::Instant,
}

#[cfg(feature = "repo-index")]
static REPO_INDEX_CACHE: Lazy<RwLock<HashMap<PathBuf, CachedIndex>>> = Lazy::new(|| RwLock::new(HashMap::new()));
// Per-root build locks to avoid duplicate concurrent builds
static REPO_BUILD_LOCKS: Lazy<RwLock<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

// repo_build_tool removed: index will auto-build on first search

pub fn repo_query_tool() -> Tool {
    Tool::new(
        REPO_QUERY_TOOL_NAME.to_string(),
        indoc! {r#"
            Search repository symbols (lazy auto-build). On first query or after TTL expiry the index
            is (re)built automatically in-memory (no on-disk artifact) unless background indexing already
            populated it. You can optionally restrict languages and request callers/callees traversal.
            TTL (seconds) can be overridden via GOOSE_REPO_INDEX_TTL_SECS (default 600). Set to 0 to disable TTL refresh.
        "#}.to_string(),
        object!({
            "type": "object",
            "required": ["root", "query"],
            "properties": {
                "root": {"type": "string", "description": "Previously indexed root"},
                "query": {"type": "string", "description": "Symbol name to search"},
                "limit": {"type": "integer", "default": 15},
                "exact_only": {"type": "boolean", "description": "Only return exact matches"},
                "min_score": {"type": "number", "description": "Minimum blended score filter (0-1)"},
                "show_score": {"type": "boolean", "description": "Include score details in result"},
                "callers_depth": {"type": "integer", "description": "Depth of reverse call traversal per match"},
                "callees_depth": {"type": "integer", "description": "Depth of forward call traversal per match"},
                "langs": {"type": "array", "items": {"type": "string"}, "description": "Optional whitelist of language IDs (e.g. rust, python)."}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("Search repository symbols".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    })
}

pub fn repo_stats_tool() -> Tool {
    Tool::new(
        REPO_STATS_TOOL_NAME.to_string(),
    "Get high-level statistics about a repository index (auto-builds if missing).".to_string(),
        object!({
            "type": "object",
            "required": ["root"],
            "properties": {
        "root": {"type": "string", "description": "Repository root path"},
        "langs": {"type": "array", "items": {"type": "string"}, "description": "Optional whitelist of language IDs if an auto-build occurs."}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("Repository index stats".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    })
}


#[cfg(feature = "repo-index")]
#[instrument(level = "info", skip(args), fields(root = %args.get("root").and_then(|v| v.as_str()).unwrap_or("?"), query = %args.get("query").and_then(|v| v.as_str()).unwrap_or("?")))]
pub async fn handle_repo_query(args: serde_json::Value) -> Result<serde_json::Value> {
    use crate::repo_index::service::StoredEntity;
    let root_s = args["root"].as_str().ok_or_else(|| anyhow!("missing root"))?;
    let query = args["query"].as_str().ok_or_else(|| anyhow!("missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(15) as usize;
    let exact_only = args["exact_only"].as_bool().unwrap_or(false);
    let min_score = args["min_score"].as_f64().unwrap_or(0.0) as f32;
    let show_score = args["show_score"].as_bool().unwrap_or(false);
    let callers_depth = args["callers_depth"].as_u64().unwrap_or(0) as u32;
    let callees_depth = args["callees_depth"].as_u64().unwrap_or(0) as u32;
    let root = Path::new(root_s).canonicalize().unwrap_or_else(|_| PathBuf::from(root_s));
    let langs: Option<Vec<String>> = args["langs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    // TTL-based auto build
    let ttl_secs: u64 = std::env::var("GOOSE_REPO_INDEX_TTL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(600);
    // Double-checked TTL + build lock
    let mut need_build = false;
    {
        let cache = REPO_INDEX_CACHE.read().await;
        if let Some(cached) = cache.get(&root) {
            if ttl_secs > 0 && cached.built_at.elapsed().as_secs() >= ttl_secs { need_build = true; }
        } else { need_build = true; }
    }
    if need_build {
        // Acquire per-root build mutex
        let lock_arc = {
            let mut locks = REPO_BUILD_LOCKS.write().await;
            locks.entry(root.clone()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
        };
        let _g = lock_arc.lock().await; // wait for any ongoing build
        // Re-check after acquiring lock
        let mut do_build = false;
        {
            let cache = REPO_INDEX_CACHE.read().await;
            if let Some(cached) = cache.get(&root) {
                if ttl_secs > 0 && cached.built_at.elapsed().as_secs() >= ttl_secs { do_build = true; }
            } else { do_build = true; }
        }
        if do_build {
            let build_start = std::time::Instant::now();
            let (svc, _stats) = {
                let mut builder_local = RepoIndexOptions::builder();
                builder_local = builder_local.root(&root);
                if let Some(lang_list) = langs.as_ref() { builder_local = builder_local.include_langs(lang_list.iter().map(|s| s.as_str()).collect()); }
                builder_local = builder_local.output_null();
                RepoIndexService::build(builder_local.build())?
            };
            let mut cache = REPO_INDEX_CACHE.write().await;
            cache.insert(root.clone(), CachedIndex { service: Arc::new(svc), built_at: std::time::Instant::now() });
            let elapsed = build_start.elapsed();
            info!(counter.goose.repo.builds = 1, event = "repo.index.build", root = %root.display(), duration_ms = elapsed.as_millis() as u64, ttl_secs, trigger = "query", "Repository index built (query path)");
        }
    }

    let svc = {
        let cache = REPO_INDEX_CACHE.read().await;
        cache.get(&root).map(|c| c.service.clone())
    }.ok_or_else(|| anyhow!("no cached index for root"))?;

    // gather candidates
    let entities: Vec<&StoredEntity> = if exact_only {
        svc.search_symbol_exact(query)
    } else {
        svc.search_symbol_fuzzy_ranked(query, limit * 2) // over-fetch a bit before min_score filter
    };

    // Recompute blended score for reporting and filtering (mirror logic in service)
    let mut min_rank = f32::MAX; let mut max_rank = f32::MIN;
    for e in &svc.entities { if e.rank < min_rank { min_rank = e.rank; } if e.rank > max_rank { max_rank = e.rank; } }
    let rank_range = if (max_rank - min_rank).abs() < 1e-9 { 1.0 } else { max_rank - min_rank };

    let mut results = Vec::new();
    for e in entities.into_iter() {
        if e.kind.as_str() == "file" { continue; }
        let name_lower = e.name.to_lowercase();
        let q_lower = query.to_lowercase();
        let mut lex = 0.0f32;
        if name_lower == q_lower { lex = 1.0; }
        else if name_lower.starts_with(&q_lower) { lex = 0.8; }
        else if name_lower.contains(&q_lower) { lex = 0.5; }
        else {
            // small inline levenshtein (duplicate ok for now)
            let dist = {
                let a = &name_lower; let b = &q_lower;
                let mut dp: Vec<usize> = (0..=b.len()).collect();
                for (i, ca) in a.chars().enumerate() {
                    let mut prev = dp[0]; dp[0] = i + 1;
                    for (j, cb) in b.chars().enumerate() {
                        let temp = dp[j + 1];
                        dp[j + 1] = if ca == cb { prev } else { 1 + prev.min(dp[j]).min(dp[j + 1]) };
                        prev = temp;
                    }
                }
                *dp.last().unwrap()
            };
            if dist <= 2 { lex = (0.3 - 0.1 * dist as f32).max(0.0); }
        }
        if lex == 0.0 && !exact_only { continue; }
        let norm_rank = (e.rank - min_rank) / rank_range;
        let blended = if exact_only { lex.max(1.0) } else { lex * 0.6 + norm_rank * 0.4 };
        if blended < min_score { continue; }

        // Optionally add graph expansion
        let mut callers = Vec::new();
        if callers_depth > 0 { callers = svc.callers_up_to(e.id, callers_depth); }
        let mut callees = Vec::new();
        if callees_depth > 0 { callees = svc.callees_up_to(e.id, callees_depth); }

        results.push(serde_json::json!({
            "id": e.id,
            "name": e.name,
            "kind": e.kind.as_str(),
            "file": svc.files[e.file_id as usize].path,
            "rank": e.rank,
            "score": if show_score { Some(blended) } else { None },
            "callers": if callers_depth>0 { Some(callers) } else { None },
            "callees": if callees_depth>0 { Some(callees) } else { None },
        }));
        if results.len() >= limit { break; }
    }

    info!(counter.goose.repo.search_calls = 1, event = "repo.index.search", root = %root.display(), query, results = results.len(), exact_only, callers_depth, callees_depth, limit, "Repository search executed");
    Ok(serde_json::json!({"results": results}))
}

#[cfg(feature = "repo-index")]
#[instrument(level = "info", skip(args), fields(root = %args.get("root").and_then(|v| v.as_str()).unwrap_or("?")))]
pub async fn handle_repo_stats(args: serde_json::Value) -> Result<serde_json::Value> {
    let root_s = args["root"].as_str().ok_or_else(|| anyhow!("missing root"))?;
    let root = Path::new(root_s).canonicalize().unwrap_or_else(|_| PathBuf::from(root_s));
    let langs: Option<Vec<String>> = args["langs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    // If not cached, build (no TTL for stats path; on-demand only)
    let exists = { REPO_INDEX_CACHE.read().await.contains_key(&root) };
    if !exists {
        let lock_arc = {
            let mut locks = REPO_BUILD_LOCKS.write().await;
            locks.entry(root.clone()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
        };
        let _g = lock_arc.lock().await;
        let exists2 = { REPO_INDEX_CACHE.read().await.contains_key(&root) };
    if !exists2 {
        let build_start = std::time::Instant::now();
            let (svc, _stats) = {
                let mut builder_local = RepoIndexOptions::builder();
                builder_local = builder_local.root(&root).output_null();
                if let Some(lang_list) = langs.as_ref() { builder_local = builder_local.include_langs(lang_list.iter().map(|s| s.as_str()).collect()); }
                RepoIndexService::build(builder_local.build())?
            };
            let mut cache = REPO_INDEX_CACHE.write().await;
            cache.insert(root.clone(), CachedIndex { service: Arc::new(svc), built_at: std::time::Instant::now() });
        let elapsed = build_start.elapsed();
            info!(counter.goose.repo.builds = 1, event = "repo.index.build", root = %root.display(), duration_ms = elapsed.as_millis() as u64, trigger = "stats", reason = "stats", "Repository index built (stats path)");
        }
    }
    let svc = {
        let cache = REPO_INDEX_CACHE.read().await;
        cache.get(&root).map(|c| c.service.clone())
    }.ok_or_else(|| anyhow!("index build failed"))?;
    info!(counter.goose.repo.stats_calls = 1, event = "repo.index.stats", root = %root.display(), files = svc.files.len(), entities = svc.entities.len(), "Repository stats collected");
    Ok(serde_json::json!({
        "root": root,
        "files": svc.files.len(),
        "entities": svc.entities.len(),
        "unresolved_imports_files": svc.unresolved_imports.iter().filter(|v| !v.is_empty()).count(),
        "rank_weights": {
            "call": svc.rank_weights.call,
            "import": svc.rank_weights.import,
            "containment": svc.rank_weights.containment,
            "damping": svc.rank_weights.damping,
            "iterations": svc.rank_weights.iterations
        }
    }))
}

#[cfg(not(feature = "repo-index"))]
pub async fn handle_repo_build(_args: serde_json::Value) -> Result<serde_json::Value, anyhow::Error> { Err(anyhow::anyhow!("repo-index feature disabled")) }
#[cfg(not(feature = "repo-index"))]
pub async fn handle_repo_query(_args: serde_json::Value) -> Result<serde_json::Value, anyhow::Error> { Err(anyhow::anyhow!("repo-index feature disabled")) }
#[cfg(not(feature = "repo-index"))]
pub async fn handle_repo_stats(_args: serde_json::Value) -> Result<serde_json::Value, anyhow::Error> { Err(anyhow::anyhow!("repo-index feature disabled")) }
