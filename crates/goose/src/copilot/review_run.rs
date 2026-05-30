use anyhow::Result;

use super::github::{
    complete_check_run, complete_check_run_failure, fetch_pr_base_sha, git_clone_and_checkout_pr,
    post_review,
};
use super::review::Finding;
use super::runner::run_goose_review;
use super::store::load_prefs;
use super::types::CopilotReviewRequest;

pub async fn run_review(req: CopilotReviewRequest) -> Result<Vec<Finding>> {
    let result = run_review_inner(&req).await;
    if let Err(e) = &result {
        if let Some(crid) = req.check_run_id {
            if let Err(complete_err) = complete_check_run_failure(&req, crid, &e.to_string()).await
            {
                tracing::warn!("[copilot] complete failed review check run: {complete_err}");
            }
        }
    }
    result
}

async fn run_review_inner(req: &CopilotReviewRequest) -> Result<Vec<Finding>> {
    let prefs = load_prefs();
    let base_sha = fetch_pr_base_sha(req).await?;
    let workdir = tempfile::tempdir()?;
    git_clone_and_checkout_pr(req, workdir.path()).await?;
    let findings = run_goose_review(workdir.path(), &base_sha, &req.head_sha, &prefs).await?;
    post_review(req, &findings, &prefs).await?;
    if let Some(crid) = req.check_run_id {
        complete_check_run(req, crid, &findings).await?;
    }
    Ok(findings)
}
