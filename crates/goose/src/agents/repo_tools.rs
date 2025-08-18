use indoc::indoc;
use once_cell::sync::Lazy;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "repo-index")]
use crate::repo_index::{RepoIndexOptions};
#[cfg(feature = "repo-index")]
use crate::repo_index::service::RepoIndexService;
#[cfg(feature = "repo-index")]
use anyhow::{anyhow, Result};

// Tool name constants
pub const REPO_BUILD_TOOL_NAME: &str = "repo__build_index";
pub const REPO_QUERY_TOOL_NAME: &str = "repo__search";
pub const REPO_STATS_TOOL_NAME: &str = "repo__stats";

#[derive(Clone)]
struct CachedIndex {
    service: Arc<RepoIndexService>,
}

#[cfg(feature = "repo-index")]
static REPO_INDEX_CACHE: Lazy<RwLock<HashMap<PathBuf, CachedIndex>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn repo_build_tool() -> Tool {
    Tool::new(
        REPO_BUILD_TOOL_NAME.to_string(),
        indoc! {r#"
            Build (or rebuild) the repository symbol index and graph for a given root directory.
            Always call this first before attempting repo__search if an index may be stale or missing.
            Provide an absolute or workspace-relative path. Optionally restrict languages.
        "#}.to_string(),
        object!({
            "type": "object",
            "required": ["root"],
            "properties": {
                "root": {"type": "string", "description": "Root directory to index"},
                "langs": {"type": "array", "items": {"type": "string"}, "description": "Optional subset of languages to include"},
                "force": {"type": "boolean", "description": "Force rebuild even if cached"}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("Build repository index".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn repo_query_tool() -> Tool {
    Tool::new(
        REPO_QUERY_TOOL_NAME.to_string(),
        indoc! {r#"
            Search repository symbols using fuzzy + rank blend and optionally expand call graph.
            Use after repo__build_index. Provide a query string; you can request callers or callees traversal.
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
                "callees_depth": {"type": "integer", "description": "Depth of forward call traversal per match"}
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
        "Get high-level statistics about an indexed repository including counts and rank weights.".to_string(),
        object!({
            "type": "object",
            "required": ["root"],
            "properties": {
                "root": {"type": "string", "description": "Indexed root"}
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
pub async fn handle_repo_build(args: serde_json::Value) -> Result<serde_json::Value> {
    let root_s = args["root"].as_str().ok_or_else(|| anyhow!("missing root"))?;
    let root = Path::new(root_s).canonicalize().unwrap_or_else(|_| PathBuf::from(root_s));
    let langs: Option<Vec<String>> = args["langs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
    let force = args["force"].as_bool().unwrap_or(false);
    // Fast path: check cache before building heavy options; clone path for Send safety
    let already_cached = {
        let cache = REPO_INDEX_CACHE.read().await;
        !force && cache.contains_key(&root)
    };
    if already_cached {
        return Ok(serde_json::json!({"status":"cached","root":root,"message":"index already cached"}));
    }

    // Build options without holding references across await points
    // Build options fully before any awaits so builder does not live across await points
    let opts = {
        let mut builder_local = RepoIndexOptions::builder();
        builder_local = builder_local.root(&root);
        if let Some(lang_list) = langs.as_ref() {
            builder_local = builder_local.include_langs(lang_list.iter().map(|s| s.as_str()).collect());
        }
    // Provide a dummy file path sink (discard output). On Unix, use /dev/null.
    #[cfg(target_family = "unix")]
    let null_path = std::path::Path::new("/dev/null");
    #[cfg(not(target_family = "unix"))]
    let null_path = root.join(".goose_index_tmp.jsonl");
    builder_local = builder_local.output_file(null_path);
        builder_local.build()
    };
    let (svc, stats) = RepoIndexService::build(opts)?; // synchronous
    let cached = CachedIndex { service: Arc::new(svc) };
    {
        let mut cache = REPO_INDEX_CACHE.write().await;
        cache.insert(root.clone(), cached);
    }
    Ok(serde_json::json!({
        "status":"built",
        "root": root,
        "files_indexed": stats.files_indexed,
        "entities_indexed": stats.entities_indexed,
        "duration_ms": stats.duration.as_millis()
    }))
}

#[cfg(feature = "repo-index")]
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

    Ok(serde_json::json!({"results": results}))
}

#[cfg(feature = "repo-index")]
pub async fn handle_repo_stats(args: serde_json::Value) -> Result<serde_json::Value> {
    let root_s = args["root"].as_str().ok_or_else(|| anyhow!("missing root"))?;
    let root = Path::new(root_s).canonicalize().unwrap_or_else(|_| PathBuf::from(root_s));
    let svc = {
        let cache = REPO_INDEX_CACHE.read().await;
        cache.get(&root).map(|c| c.service.clone())
    }.ok_or_else(|| anyhow!("no cached index for root"))?;
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
