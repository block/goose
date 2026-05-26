//! Multi-model debate orchestrator for `goose review`.
//!
//! Runs the same diff through N debaters (provider/model pairs) over R
//! rounds. In round 1 each debater reviews independently; in rounds
//! 2..=R each debater is shown the anonymized findings from every other
//! debater in the previous round and must keep / drop / refine each
//! claim with cited evidence, or add new claims. After the last round,
//! findings are aggregated by `(path, line_start, line_end)` and only
//! kept when at least `min_agreement` debaters retained the claim.
//!
//! Modeled after the methodology in
//! <https://milvus.io/blog/ai-code-review-gets-better-when-models-debate-claude-vs-gemini-vs-codex-vs-qwen-vs-minimax.md>
//! which reported a 53% → 80% bug-recall lift moving from a single
//! model to a 5-model × 5-round debate.
//!
//! The strict `{"findings": [...]}` JSON contract is reused from the
//! single-model orchestrator, so the JSONL emitted on stdout stays
//! shape-compatible with the existing `gooseFinding` parser in
//! squareup/agents' `review_goose.go`. New optional `round` and
//! `agreement` fields are populated for debate-produced findings and
//! skip-serialized when empty.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::handler::ReviewOptions;
use super::orchestrator::{
    run_subprocess_for_findings, split_diff_by_file, Finding, RawFinding, MAX_WORKERS,
};

/// One debate participant. The `label` is an opaque short token
/// (A, B, C, ...) used when showing this debater's findings to its
/// peers — we never surface the real provider/model name in the
/// peer-review prompt because Milvus found that smaller models
/// defer to recognized larger ones when identified, biasing the
/// debate.
#[derive(Debug, Clone)]
pub struct Debater {
    pub provider: String,
    pub model: String,
    pub label: String,
}

/// Debate orchestration knobs. Populated by the CLI layer when
/// `--debate-models` is passed.
#[derive(Debug, Clone)]
pub struct DebateOptions {
    pub debaters: Vec<Debater>,
    /// Total rounds, including round 1 (the independent pass).
    /// Round 1 alone is just N independent reviewers; rounds ≥ 2
    /// add the cross-pollination step. Capped at 5 to keep the
    /// cost ceiling predictable.
    pub rounds: u32,
    /// Minimum number of debaters that must keep a finding in the
    /// final round for it to be emitted. Default 1 mirrors "union
    /// of all reviewers"; bumping to ceil(N/2) requires majority
    /// agreement.
    pub min_agreement: usize,
}

/// Parse `--debate-models provider/model,provider/model,...` into
/// labeled [`Debater`] entries. Labels are auto-assigned A, B, C, ...
/// in source order. Returns an error if any entry is malformed or
/// fewer than two debaters are listed — a single-debater "debate"
/// would just be the single-model main pass with extra subprocess
/// overhead.
pub fn parse_debaters(spec: &str) -> Result<Vec<Debater>> {
    let mut out = Vec::new();
    for (idx, raw) in spec.split(',').enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (provider, model) = trimmed.split_once('/').ok_or_else(|| {
            anyhow!(
                "--debate-models entry {:?} must be in `provider/model` form (e.g. `google/gemini-3.1-pro-preview`)",
                trimmed
            )
        })?;
        let provider = provider.trim();
        let model = model.trim();
        if provider.is_empty() || model.is_empty() {
            return Err(anyhow!(
                "--debate-models entry {:?} has an empty provider or model",
                trimmed
            ));
        }
        out.push(Debater {
            provider: provider.to_string(),
            model: model.to_string(),
            label: label_for_index(idx),
        });
    }
    if out.len() < 2 {
        return Err(anyhow!(
            "--debate-models requires at least two `provider/model` entries; got {}",
            out.len()
        ));
    }
    Ok(out)
}

/// Map a 0-indexed debater position to its opaque label.
/// 0 → "A", 1 → "B", ..., 25 → "Z", 26 → "AA", ... so the debate
/// keeps working past 26 debaters without panicking even though we
/// don't expect to ever go that wide in practice.
fn label_for_index(idx: usize) -> String {
    let mut n = idx + 1;
    let mut chars = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        chars.push((b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    chars.iter().rev().collect()
}

/// Run the multi-model debate orchestrator.
///
/// Wall-clock is `R rounds × max(per-file × per-debater latency)`
/// because the inner per-(file,debater) fan-out runs concurrently up
/// to the [`max_concurrency`] cap. Rounds themselves are sequential —
/// round N's prompt depends on round N-1's outputs — so total cost is
/// `R × N debaters × M files × per-call cost`.
pub async fn run_debate_review(
    diff: &str,
    base_prompt: &str,
    opts: &ReviewOptions,
    debate: &DebateOptions,
) -> Vec<Finding> {
    let per_file = split_diff_by_file(diff);
    if per_file.is_empty() {
        return Vec::new();
    }

    let rounds = debate.rounds.max(1);
    let concurrency = max_concurrency(debate.debaters.len());

    if !opts.quiet {
        eprintln!(
            "goose review: debate orchestrator with {} debater(s) over {} round(s) (concurrency cap {})",
            debate.debaters.len(),
            rounds,
            concurrency
        );
    }

    // Per-file aggregation: round_findings[file_idx][debater_idx]
    // holds the Vec<Finding> that debater emitted in the most recent
    // round for that file.
    let mut per_file_per_debater: Vec<Vec<Vec<Finding>>> =
        vec![vec![Vec::new(); debate.debaters.len()]; per_file.len()];

    for round in 1..=rounds {
        let next = run_one_round(
            round,
            rounds,
            &per_file,
            &per_file_per_debater,
            base_prompt,
            opts,
            debate,
            concurrency,
        )
        .await;
        per_file_per_debater = next;
        if !opts.quiet {
            let kept: usize = per_file_per_debater
                .iter()
                .flat_map(|by_dbt| by_dbt.iter().map(|v| v.len()))
                .sum();
            eprintln!(
                "goose review: debate round {round}/{rounds} produced {kept} raw finding(s) across all debaters"
            );
        }
    }

    aggregate_by_agreement(
        &per_file_per_debater,
        &debate.debaters,
        debate.min_agreement,
    )
}

/// Run a single round of the debate, returning `[file_idx][debater_idx]
/// -> Vec<Finding>` for the next round's prompt.
#[allow(clippy::too_many_arguments)]
async fn run_one_round(
    round: u32,
    total_rounds: u32,
    per_file: &[(String, String)],
    prior: &[Vec<Vec<Finding>>],
    base_prompt: &str,
    opts: &ReviewOptions,
    debate: &DebateOptions,
    concurrency: usize,
) -> Vec<Vec<Vec<Finding>>> {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut set: JoinSet<(usize, usize, Result<Vec<RawFinding>>)> = JoinSet::new();

    for (file_idx, (path, file_diff)) in per_file.iter().enumerate() {
        for (dbt_idx, dbt) in debate.debaters.iter().enumerate() {
            let sem = semaphore.clone();
            let path = path.clone();
            let file_diff = file_diff.clone();
            let dbt = dbt.clone();
            let base_prompt = base_prompt.to_string();
            let instructions = opts.instructions.clone();
            // For round ≥ 2, materialize the anonymized peer view that
            // THIS debater will see for THIS file. Skipping self in the
            // peer block avoids the model just rubber-stamping its own
            // round-1 list.
            let peer_view: Vec<(String, Vec<Finding>)> = if round == 1 {
                Vec::new()
            } else {
                prior[file_idx]
                    .iter()
                    .enumerate()
                    .filter(|(other_idx, _)| *other_idx != dbt_idx)
                    .map(|(other_idx, findings)| {
                        (debate.debaters[other_idx].label.clone(), findings.clone())
                    })
                    .collect()
            };
            let self_prior: Vec<Finding> = if round == 1 {
                Vec::new()
            } else {
                prior[file_idx][dbt_idx].clone()
            };

            set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore is never closed");
                let prompt = build_debate_round_prompt(
                    round,
                    total_rounds,
                    &path,
                    &file_diff,
                    &base_prompt,
                    instructions.as_deref(),
                    &self_prior,
                    &peer_view,
                );
                let label = format!("debate:r{round}:{}:{}", dbt.label, path);
                let result = run_subprocess_for_findings(
                    &prompt,
                    &label,
                    Some(&dbt.provider),
                    Some(&dbt.model),
                    None,
                )
                .await;
                (file_idx, dbt_idx, result)
            });
        }
    }

    let mut next: Vec<Vec<Vec<Finding>>> =
        vec![vec![Vec::new(); debate.debaters.len()]; per_file.len()];

    while let Some(joined) = set.join_next().await {
        let (file_idx, dbt_idx, result) = match joined {
            Ok(v) => v,
            Err(e) => {
                eprintln!("goose review: debate task panicked: {e}");
                continue;
            }
        };
        match result {
            Ok(raw) => {
                let path = per_file[file_idx].0.clone();
                next[file_idx][dbt_idx] = raw
                    .into_iter()
                    .map(|r| Finding {
                        severity: r.severity.unwrap_or_else(|| "medium".to_string()),
                        path: r.path.unwrap_or_else(|| path.clone()),
                        line_start: r.line_start.unwrap_or(0),
                        line_end: r.line_end.unwrap_or(0),
                        summary: r.summary.unwrap_or_default(),
                        check: "main".to_string(),
                        round: Some(round),
                        ..Finding::default()
                    })
                    .collect();
            }
            Err(e) => {
                // Per-(file,debater) failure must never abort the
                // round — surface a warning, leave next[file][dbt]
                // empty, and continue. The aggregation step naturally
                // demotes the missing slot to "no vote".
                if !opts.quiet {
                    eprintln!(
                        "goose review: debate r{round} {} on '{}' failed: {e}",
                        debate.debaters[dbt_idx].label, per_file[file_idx].0
                    );
                }
            }
        }
    }

    next
}

/// Scale the orchestrator's semaphore by the debater count. Each
/// debater hits a different provider so the rate-limit budget is
/// effectively per-provider rather than shared. We let `N debaters ×
/// MAX_WORKERS / 2` ride above [`MAX_WORKERS`] but keep the absolute
/// ceiling at 16 to protect local CPU / memory.
fn max_concurrency(debaters: usize) -> usize {
    let scaled = (MAX_WORKERS * debaters.max(1)).div_ceil(2);
    scaled.clamp(MAX_WORKERS, 16)
}

/// Build the strict, JSON-only prompt for one (file, debater, round)
/// tuple. Round 1 is just the standard per-file main-pass prompt with
/// no peer context. Rounds ≥ 2 prepend an anonymized "Other reviewers
/// said" block and a re-review rubric.
#[allow(clippy::too_many_arguments)]
fn build_debate_round_prompt(
    round: u32,
    total_rounds: u32,
    path: &str,
    file_diff: &str,
    base_prompt: &str,
    instructions: Option<&str>,
    self_prior: &[Finding],
    peer_view: &[(String, Vec<Finding>)],
) -> String {
    let mut s = String::new();
    s.push_str(base_prompt.trim_end());
    s.push_str("\n\n");

    if let Some(text) = instructions {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            s.push_str("## Reviewer instructions\n\n");
            s.push_str(trimmed);
            s.push_str("\n\n");
        }
    }

    s.push_str("## File under review\n\n");
    s.push_str(&format!("Path: `{path}`\n\n"));

    if round == 1 {
        s.push_str(
            "Review ONLY the changes in this file. Walk every added/modified line. \
             Do not flag pre-existing code shown for context (lines beginning with a space). \
             Use post-change line numbers from the diff.\n\n",
        );
    } else {
        s.push_str(&format!(
            "## Debate round {round}/{total_rounds}\n\n\
             You previously reviewed this file. Anonymous peer reviewers ALSO reviewed it. \
             Each reviewer's findings below are independent — they could not see each other's \
             work in round 1. Now you are being asked to revise your position.\n\n\
             Your task this round:\n\
             - **Keep** a finding (yours or a peer's) only if you can cite a specific changed \
               line in the diff that supports it.\n\
             - **Drop** any finding that you cannot defend against the diff (pre-existing code, \
               wrong line range, no actual hazard).\n\
             - **Refine** a finding by emitting it with a corrected severity, line range, or \
               summary.\n\
             - **Add** any new finding you missed earlier.\n\n\
             Do not agree just to agree. If a peer is wrong, drop their claim. \
             Every claim in your output must point to a specific changed line.\n\n",
        ));
        s.push_str("### Your previous round findings\n\n");
        s.push_str(&render_findings_block(self_prior));
        s.push('\n');
        for (label, findings) in peer_view {
            s.push_str(&format!("### Reviewer {label}\n\n"));
            s.push_str(&render_findings_block(findings));
            s.push('\n');
        }
    }

    s.push_str(
        "## Output\n\nReturn ONLY valid JSON with this exact schema:\n\n\
{\n  \"findings\": [\n    {\n      \"severity\": \"low|medium|high|critical\",\n      \"path\": \"relative/path/to/file\",\n      \"line_start\": 10,\n      \"line_end\": 12,\n      \"summary\": \"One-paragraph actionable explanation of the issue and the fix\"\n    }\n  ]\n}\n\nIf there are no real issues, return:\n{\"findings\":[]}\n\nDo NOT include any text before or after the JSON. Do NOT wrap the JSON in code fences.\n\n",
    );
    s.push_str("## Diff\n\n```diff\n");
    s.push_str(file_diff.trim_end_matches('\n'));
    s.push_str("\n```\n");
    s
}

/// Render a debater's findings as a compact JSONL block for inclusion
/// in another debater's prompt. Empty list emits the bare `[]` token so
/// the prompt is unambiguous about "this reviewer found nothing".
fn render_findings_block(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "[]\n".to_string();
    }
    let mut s = String::new();
    for f in findings {
        // Use the strict schema (no `check`, no debate metadata) so the
        // peer model doesn't try to copy them through. serde_json never
        // fails on owned strings + numbers.
        let payload = serde_json::json!({
            "severity": f.severity,
            "path": f.path,
            "line_start": f.line_start,
            "line_end": f.line_end,
            "summary": f.summary,
        });
        s.push_str(&payload.to_string());
        s.push('\n');
    }
    s
}

/// Cluster findings across debaters (final round only) by
/// `(path, line_start, line_end)` and emit one consolidated [`Finding`]
/// per cluster when at least `min_agreement` debaters kept the claim.
///
/// The consolidated severity is the max across keepers (any high
/// trumps medium); the summary is the longest keeper's text (longest
/// = most explanatory in practice); the `agreement` field carries the
/// sorted list of keeper labels.
fn aggregate_by_agreement(
    per_file_per_debater: &[Vec<Vec<Finding>>],
    debaters: &[Debater],
    min_agreement: usize,
) -> Vec<Finding> {
    type ClusterKey = (String, i64, i64);
    // BTreeMap keeps emit order deterministic across runs, which keeps
    // test fixtures stable and downstream cache keys reproducible.
    let mut clusters: BTreeMap<ClusterKey, Vec<(String, Finding)>> = BTreeMap::new();

    for by_debater in per_file_per_debater {
        for (dbt_idx, findings) in by_debater.iter().enumerate() {
            let label = debaters[dbt_idx].label.clone();
            for f in findings {
                let key = (f.path.clone(), f.line_start, f.line_end);
                clusters
                    .entry(key)
                    .or_default()
                    .push((label.clone(), f.clone()));
            }
        }
    }

    let threshold = min_agreement.max(1);
    let mut out = Vec::new();
    for ((_path, _start, _end), votes) in clusters {
        // Multiple findings from the same debater for the same
        // (path,line) cluster only count once toward agreement —
        // otherwise a verbose debater could outvote everyone else by
        // emitting duplicates.
        let mut unique_labels: Vec<String> = votes.iter().map(|(l, _)| l.clone()).collect();
        unique_labels.sort();
        unique_labels.dedup();
        if unique_labels.len() < threshold {
            continue;
        }
        let merged = merge_cluster(votes);
        out.push(Finding {
            agreement: Some(unique_labels),
            ..merged
        });
    }
    out
}

/// Pick the representative finding from a cluster: highest severity
/// wins; tie-broken by the longest summary (most explanatory).
fn merge_cluster(votes: Vec<(String, Finding)>) -> Finding {
    votes
        .into_iter()
        .map(|(_, f)| f)
        .max_by(|a, b| {
            severity_rank(&a.severity)
                .cmp(&severity_rank(&b.severity))
                .then_with(|| a.summary.len().cmp(&b.summary.len()))
        })
        .expect("cluster has at least one vote (filtered by threshold above)")
}

fn severity_rank(s: &str) -> u8 {
    match s.trim().to_ascii_lowercase().as_str() {
        "low" | "info" | "note" => 0,
        "medium" | "med" | "" => 1,
        "high" => 2,
        "critical" | "crit" | "blocker" => 3,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dbt(label: &str, provider: &str, model: &str) -> Debater {
        Debater {
            provider: provider.to_string(),
            model: model.to_string(),
            label: label.to_string(),
        }
    }

    fn finding(label_severity: &str, path: &str, line: i64, summary: &str) -> Finding {
        Finding {
            severity: label_severity.to_string(),
            path: path.to_string(),
            line_start: line,
            line_end: line,
            summary: summary.to_string(),
            check: "main".to_string(),
            ..Finding::default()
        }
    }

    #[test]
    fn label_for_index_runs_past_z() {
        assert_eq!(label_for_index(0), "A");
        assert_eq!(label_for_index(1), "B");
        assert_eq!(label_for_index(25), "Z");
        assert_eq!(label_for_index(26), "AA");
        assert_eq!(label_for_index(27), "AB");
        assert_eq!(label_for_index(51), "AZ");
        assert_eq!(label_for_index(52), "BA");
    }

    #[test]
    fn parse_debaters_accepts_three_entries_with_whitespace() {
        let v = parse_debaters(
            "google/gemini-3.1-pro-preview, anthropic/claude-sonnet-4.5 , openai/gpt-5.5",
        )
        .unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].provider, "google");
        assert_eq!(v[0].model, "gemini-3.1-pro-preview");
        assert_eq!(v[0].label, "A");
        assert_eq!(v[1].label, "B");
        assert_eq!(v[2].provider, "openai");
        assert_eq!(v[2].label, "C");
    }

    #[test]
    fn parse_debaters_rejects_fewer_than_two() {
        let err = parse_debaters("google/gemini-3.1-pro-preview").unwrap_err();
        assert!(err.to_string().contains("at least two"));
    }

    #[test]
    fn parse_debaters_rejects_missing_slash() {
        let err = parse_debaters("google/gemini-3.1,broken").unwrap_err();
        assert!(err.to_string().contains("provider/model"));
    }

    #[test]
    fn parse_debaters_rejects_empty_provider_or_model() {
        assert!(parse_debaters("google/,anthropic/claude").is_err());
        assert!(parse_debaters("/gemini,anthropic/claude").is_err());
    }

    #[test]
    fn parse_debaters_skips_empty_entries_from_trailing_commas() {
        let v = parse_debaters("google/gemini,anthropic/claude,").unwrap();
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn max_concurrency_grows_with_debaters_but_caps_at_16() {
        // MAX_WORKERS is 4 today; ceil(4*N/2) clamped to [4,16].
        assert_eq!(max_concurrency(1), MAX_WORKERS);
        assert_eq!(max_concurrency(2), MAX_WORKERS);
        assert_eq!(max_concurrency(3), 6);
        assert_eq!(max_concurrency(5), 10);
        assert_eq!(max_concurrency(8), 16);
        assert_eq!(max_concurrency(20), 16);
    }

    #[test]
    fn aggregate_emits_finding_when_min_agreement_met() {
        let debaters = vec![
            dbt("A", "p1", "m1"),
            dbt("B", "p2", "m2"),
            dbt("C", "p3", "m3"),
        ];
        // file 0: A and B both flag line 42; C disagrees with a
        // different line. min_agreement=2 should keep the (42) cluster
        // and drop C's (99) cluster.
        let per_file = vec![vec![
            vec![finding("high", "foo.rs", 42, "leak under err path")],
            vec![finding("medium", "foo.rs", 42, "fd not closed")],
            vec![finding("low", "foo.rs", 99, "nit")],
        ]];
        let out = aggregate_by_agreement(&per_file, &debaters, 2);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, "foo.rs");
        assert_eq!(out[0].line_start, 42);
        // Max severity wins across keepers.
        assert_eq!(out[0].severity, "high");
        // Labels are sorted + deduplicated.
        assert_eq!(
            out[0].agreement.as_deref(),
            Some(&["A".into(), "B".into()][..])
        );
    }

    #[test]
    fn aggregate_dedupes_same_debater_repeats_before_counting_agreement() {
        let debaters = vec![dbt("A", "p1", "m1"), dbt("B", "p2", "m2")];
        // A votes twice for the same (path,line); without dedup that
        // would meet min_agreement=2 by itself. Should NOT emit.
        let per_file = vec![vec![
            vec![
                finding("high", "foo.rs", 10, "first"),
                finding("medium", "foo.rs", 10, "duplicate"),
            ],
            vec![],
        ]];
        let out = aggregate_by_agreement(&per_file, &debaters, 2);
        assert!(out.is_empty(), "self-duplicates must not satisfy agreement");
    }

    #[test]
    fn aggregate_with_min_agreement_one_acts_like_union() {
        let debaters = vec![dbt("A", "p1", "m1"), dbt("B", "p2", "m2")];
        let per_file = vec![vec![
            vec![finding("high", "foo.rs", 1, "a-only")],
            vec![finding("low", "foo.rs", 2, "b-only")],
        ]];
        let out = aggregate_by_agreement(&per_file, &debaters, 1);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn aggregate_picks_longest_summary_on_severity_tie() {
        let debaters = vec![dbt("A", "p1", "m1"), dbt("B", "p2", "m2")];
        let per_file = vec![vec![
            vec![finding("medium", "foo.rs", 5, "short")],
            vec![finding(
                "medium",
                "foo.rs",
                5,
                "much longer explanation of the same hazard",
            )],
        ]];
        let out = aggregate_by_agreement(&per_file, &debaters, 2);
        assert_eq!(out.len(), 1);
        assert!(out[0].summary.contains("longer explanation"));
    }

    #[test]
    fn severity_rank_orders_correctly() {
        assert!(severity_rank("low") < severity_rank("medium"));
        assert!(severity_rank("medium") < severity_rank("high"));
        assert!(severity_rank("high") < severity_rank("critical"));
        assert_eq!(severity_rank("BLOCKER"), severity_rank("critical"));
        assert_eq!(severity_rank("info"), severity_rank("low"));
    }

    #[test]
    fn render_findings_block_uses_strict_schema_with_no_check_or_metadata() {
        let f = finding("high", "foo.rs", 42, "leak");
        let mut with_meta = f.clone();
        with_meta.round = Some(2);
        with_meta.agreement = Some(vec!["A".into(), "C".into()]);
        let block = render_findings_block(&[with_meta]);
        assert!(block.contains("\"severity\":\"high\""));
        assert!(block.contains("\"path\":\"foo.rs\""));
        assert!(block.contains("\"line_start\":42"));
        // The peer prompt must NOT leak the `check`, `round`, or
        // `agreement` fields — they'd confuse the model into trying
        // to copy them into its own output.
        assert!(!block.contains("\"check\""));
        assert!(!block.contains("\"round\""));
        assert!(!block.contains("\"agreement\""));
    }

    #[test]
    fn render_findings_block_empty_yields_bracketed_marker() {
        assert_eq!(render_findings_block(&[]), "[]\n");
    }

    #[test]
    fn build_round_one_prompt_has_no_peer_block_or_debate_header() {
        let prompt = build_debate_round_prompt(
            1,
            3,
            "src/foo.rs",
            "diff --git a/src/foo.rs b/src/foo.rs\n@@ -1 +1 @@\n-x\n+y\n",
            "BASE PROMPT",
            None,
            &[],
            &[],
        );
        assert!(prompt.starts_with("BASE PROMPT"));
        assert!(prompt.contains("Path: `src/foo.rs`"));
        assert!(!prompt.contains("Debate round"));
        assert!(!prompt.contains("### Reviewer"));
        assert!(prompt.contains("Return ONLY valid JSON"));
    }

    #[test]
    fn build_round_two_prompt_anonymizes_peers_and_demands_evidence() {
        let self_prior = vec![finding("high", "src/foo.rs", 10, "my prior claim")];
        let peer_view = vec![
            (
                "B".into(),
                vec![finding("low", "src/foo.rs", 20, "B's claim")],
            ),
            ("C".into(), Vec::new()),
        ];
        let prompt = build_debate_round_prompt(
            2,
            3,
            "src/foo.rs",
            "diff body",
            "BASE",
            Some("PR intent: refactor only"),
            &self_prior,
            &peer_view,
        );
        assert!(prompt.contains("## Debate round 2/3"));
        assert!(prompt.contains("### Your previous round findings"));
        assert!(prompt.contains("### Reviewer B"));
        assert!(prompt.contains("### Reviewer C"));
        // C had zero prior findings; the empty marker keeps the prompt
        // unambiguous.
        assert!(prompt.contains("[]"));
        assert!(prompt.contains("PR intent: refactor only"));
        // No real provider/model names should appear in the peer view.
        assert!(!prompt.contains("google"));
        assert!(!prompt.contains("anthropic"));
        // The keep/drop/refine rubric must be present.
        assert!(prompt.contains("**Keep**"));
        assert!(prompt.contains("**Drop**"));
        assert!(prompt.contains("**Refine**"));
        assert!(prompt.contains("**Add**"));
    }

    #[test]
    fn build_round_two_prompt_omits_self_block_when_self_prior_empty() {
        // First-round empty (no findings) → round 2 still renders the
        // "Your previous round findings" header with `[]`, never panics.
        let prompt = build_debate_round_prompt(
            2,
            2,
            "src/foo.rs",
            "diff",
            "BASE",
            None,
            &[],
            &[("B".into(), vec![finding("medium", "src/foo.rs", 1, "x")])],
        );
        assert!(prompt.contains("### Your previous round findings\n\n[]"));
    }
}
