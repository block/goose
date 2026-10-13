use super::super::{forecast, lineage, quality, suitability, transport::Hub};
use super::*;
use anyhow::Context;

use futures::{stream, StreamExt};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

// Elect representatives by popularity within each evaluated checkpoint and conversion class.
// This bounds duplicate publishers, never checkpoint sizes or the number of conversion classes.
const MAX_ARTIFACT_CANDIDATES_PER_CLASS: usize = 3;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    evaluated_checkpoints: usize,
    artifact_candidates_per_class: usize,
    candidates: usize,
    models: usize,
    variants: usize,
    exclusions: Vec<Exclusion>,
    errors: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Exclusion {
    repo: String,
    variant: Option<String>,
    reason: String,
}

fn transient(error: &anyhow::Error) -> bool {
    if error.chain().any(|cause| cause.is::<std::io::Error>()) {
        return true;
    }
    error
        .chain()
        .filter_map(|error| error.downcast_ref::<reqwest::Error>())
        .any(|error| {
            error
                .status()
                .is_none_or(|status| status.is_server_error() || status.as_u16() == 429)
        })
}

async fn inspect_repo(
    hub: &Hub,
    index: &quality::QualityIndex,
    repo: &str,
    covered: &mut BTreeSet<(String, String)>,
    attempted: &mut BTreeMap<(String, String), usize>,
) -> Result<(Option<Model>, Vec<Exclusion>)> {
    let manifest = hub.model(repo).await?;
    let revision = manifest["sha"]
        .as_str()
        .filter(|sha| valid_revision(sha))
        .context("Missing immutable revision")?;
    let quality = index
        .for_model(repo, &manifest)
        .context("No unambiguous benchmark lineage")?;
    let parent = if quality.inherited {
        Some(lineage::evaluated(hub, repo, &manifest, &quality.evaluated_model_id).await?)
    } else {
        None
    };
    let (purpose, reason) = suitability::assess_checked(hub, &manifest, Some(&quality)).await?;
    let mut excluded = Vec::new();
    if purpose == Suitability::Unsuitable {
        return Ok((
            None,
            vec![Exclusion {
                repo: repo.into(),
                variant: None,
                reason,
            }],
        ));
    }
    let variants = crate::hf_models::variants_from_manifest(repo, &manifest)?;
    let mut accepted = Vec::new();
    for mut variant in variants {
        let class = (variant.format.clone(), variant.variant_id.clone());
        if covered.contains(&class)
            || attempted.get(&class).copied().unwrap_or(0) >= MAX_ARTIFACT_CANDIDATES_PER_CLASS
        {
            continue;
        }
        *attempted.entry(class.clone()).or_default() += 1;
        eprintln!("Inspecting {repo}:{}", variant.variant_id);
        let result = async {
            let lineage = match &parent {
                Some(parent) => Some(lineage::verify(hub, repo, revision, &variant, parent).await?),
                None => None,
            };
            let metadata =
                forecast::fetch_metadata(hub, repo, revision, &variant, &manifest).await?;
            Ok((super::worker::inspect(metadata).await?, lineage))
        }
        .await;
        let (memory, assumptions, embedded_template, lineage) = match result {
            Ok((
                super::worker::Outcome::Compatible {
                    memory,
                    assumptions,
                    chat_template,
                },
                lineage,
            )) => (memory, assumptions, chat_template, lineage),
            Ok((super::worker::Outcome::Unverified(reason), _)) => {
                excluded.push(Exclusion {
                    repo: repo.into(),
                    variant: Some(variant.variant_id),
                    reason,
                });
                continue;
            }
            Ok((super::worker::Outcome::Incompatible(reasons), _)) => {
                eprintln!(
                    "Rejected {repo}:{}: {:.240} ({} issues)",
                    variant.variant_id,
                    reasons
                        .first()
                        .map(String::as_str)
                        .unwrap_or("Compatibility not established"),
                    reasons.len()
                );
                excluded.push(Exclusion {
                    repo: repo.into(),
                    variant: Some(variant.variant_id),
                    reason: reasons.join("; "),
                });
                continue;
            }
            Err(error) if transient(&error) => {
                eprintln!("Source failure {repo}:{}: {error:#}", variant.variant_id);
                return Err(error);
            }
            Err(error) => {
                eprintln!("Unresolved {repo}:{}: {error}", variant.variant_id);
                excluded.push(Exclusion {
                    repo: repo.into(),
                    variant: Some(variant.variant_id),
                    reason: format!("Unresolved inspection: {error}"),
                });
                continue;
            }
        };
        let template = purpose == Suitability::Suitable
            || embedded_template
            || suitability::fetch_chat_template_checked(hub, repo, revision, &manifest).await?;
        let (suitability, evidence) = if purpose == Suitability::Suitable {
            (purpose, reason.clone())
        } else {
            suitability::classify(&manifest, Some(&quality), template)
        };
        if suitability != Suitability::Suitable {
            excluded.push(Exclusion {
                repo: repo.into(),
                variant: Some(variant.variant_id),
                reason: evidence,
            });
            continue;
        }
        let (files, projector) = crate::hf_models::catalogue_files(repo, &variant, &manifest)?;

        if let Some(filename) = &variant.filename {
            variant.download_url =
                Some(Hub::url(&format!("{repo}/resolve/{revision}/{filename}"), &[])?.into());
        }
        accepted.push(Variant {
            lineage,
            variant,
            files,
            projector,
            suitability_evidence: vec![evidence],
            forecast_assumptions: assumptions,
            memory,
        });
    }
    if accepted.is_empty() {
        excluded.push(Exclusion {
            repo: repo.into(),
            variant: None,
            reason: "No eligible local variants".into(),
        });
        return Ok((None, excluded));
    }
    accepted.sort_by(|a, b| a.variant.variant_id.cmp(&b.variant.variant_id));
    for item in &accepted {
        eprintln!("Accepted {repo}:{}", item.variant.variant_id);
        covered.insert((item.variant.format.clone(), item.variant.variant_id.clone()));
    }
    Ok((
        Some(Model {
            repo_id: repo.into(),
            revision: revision.into(),
            downloads: manifest["downloads"].as_u64().unwrap_or(0),
            quality,
            variants: accepted,
        }),
        excluded,
    ))
}

pub(super) fn verify_memory_fit(
    request: &eredu::api::GenerationMemoryRequest,
    memory: &MemoryForecast,
) -> Result<()> {
    use eredu::api::{estimate_generation_memory, MemoryFit};
    let mut request = request.clone();
    for available in [
        0,
        memory
            .lower_bytes
            .saturating_add(RESERVE_BYTES)
            .saturating_sub(1),
        memory
            .upper_bytes
            .unwrap_or(memory.lower_bytes)
            .saturating_add(RESERVE_BYTES),
    ] {
        request.domains[0].budget.available_bytes = (available > 0).then_some(available);
        request.domains[0].budget.reserve_bytes = RESERVE_BYTES;
        let expected = match estimate_generation_memory(&request)?.fit {
            MemoryFit::LikelyFit => Fit::LikelyFit,
            MemoryFit::LikelyShortfall => Fit::LikelyShortfall,
            MemoryFit::InsufficientInformation => Fit::Unknown,
        };
        ensure!(
            memory_fit(memory, available) == expected,
            "Stored forecast bounds disagree with eredu fit"
        );
    }
    Ok(())
}

pub async fn rebuild(path: &Path, report_path: &Path) -> Result<()> {
    let hub = Hub::public(Some(report_path.with_extension("metadata")))?;
    let index = quality::QualityIndex::fetch().await?;
    let mut families = Vec::new();
    let mut candidate_count = 0;
    for parent in &index.candidates {
        let mut candidates = quality::derived_models(&hub, parent)
            .await
            .with_context(|| format!("Discovering quantizations of {parent}"))?;
        // Original checkpoints and every conversion class remain eligible regardless of size.
        candidates.insert(0, serde_json::json!({"id":parent}));
        candidate_count += candidates.len();
        families.push(candidates);
    }
    eprintln!(
        "Inspecting {candidate_count} repositories from {} evaluated checkpoints",
        index.candidates.len()
    );
    let mut report = Report {
        evaluated_checkpoints: index.candidates.len(),
        artifact_candidates_per_class: MAX_ARTIFACT_CANDIDATES_PER_CLASS,
        candidates: candidate_count,
        models: 0,
        variants: 0,
        exclusions: Vec::new(),
        errors: Vec::new(),
    };
    let mut models = BTreeMap::<String, Model>::new();
    let cache_dir = report_path.with_extension("cache");
    std::fs::create_dir_all(&cache_dir)?;
    let requests = families
        .into_iter()
        .map(|candidates| {
            let (hub, index, cache_dir) = (&hub, &index, &cache_dir);
            async move {
                use std::hash::{Hash, Hasher};
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                (
                    eredu_revision(),
                    POLICY_VERSION,
                    serde_json::to_string(&Profile::default()).unwrap(),
                    serde_json::to_string(
                        &candidates
                            .iter()
                            .map(|candidate| (&candidate["id"], &candidate["sha"]))
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                    serde_json::to_string(&index.models.iter().collect::<BTreeMap<_, _>>())
                        .unwrap(),
                )
                    .hash(&mut hash);
                let cache_path = cache_dir.join(format!("{:016x}.json", hash.finish()));
                // Cache is short-lived: an interrupted refresh can resume, but releases re-read mutable HF metadata.
                if let Ok(metadata) = std::fs::metadata(&cache_path) {
                    if metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.elapsed().ok())
                        .is_some_and(|age| age.as_secs() < 24 * 60 * 60)
                    {
                        if let Ok(bytes) = std::fs::read(&cache_path) {
                            if let Ok(cached) = serde_json::from_slice::<
                                Vec<(String, (Option<Model>, Vec<Exclusion>))>,
                            >(&bytes)
                            {
                                return cached
                                    .into_iter()
                                    .map(|(repo, value)| (repo, Ok(value)))
                                    .collect();
                            }
                        }
                    }
                }
                let mut covered = BTreeSet::new();
                let mut attempted = BTreeMap::new();
                let mut outcomes = Vec::new();
                for candidate in candidates {
                    let Some(repo) = candidate["id"].as_str() else {
                        continue;
                    };
                    if candidate["siblings"].is_array() {
                        let variants = crate::hf_models::variants_from_manifest(repo, &candidate);
                        if variants.as_ref().is_ok_and(|variants| {
                            variants.iter().all(|variant| {
                                let class = (variant.format.clone(), variant.variant_id.clone());
                                covered.contains(&class)
                                    || attempted.get(&class).copied().unwrap_or(0)
                                        >= MAX_ARTIFACT_CANDIDATES_PER_CLASS
                            })
                        }) {
                            continue;
                        }
                    }
                    let saved_covered = covered.clone();
                    let saved_attempted = attempted.clone();
                    let mut result =
                        inspect_repo(hub, index, repo, &mut covered, &mut attempted).await;
                    // Response bodies can fail after HTTP headers have already succeeded.
                    // Restore the election budget so retries cannot silently omit variants.
                    for retry in 0..2 {
                        if !result.as_ref().is_err_and(transient) {
                            break;
                        }
                        eprintln!("Retrying interrupted inspection of {repo}");
                        tokio::time::sleep(std::time::Duration::from_secs(2 << retry)).await;
                        covered = saved_covered.clone();
                        attempted = saved_attempted.clone();
                        result = inspect_repo(hub, index, repo, &mut covered, &mut attempted).await;
                    }
                    let result = match result {
                        Err(error) if !transient(&error) => Ok((
                            None,
                            vec![Exclusion {
                                repo: repo.into(),
                                variant: None,
                                reason: error.to_string(),
                            }],
                        )),
                        result => result,
                    };
                    let source_failed = result.is_err();
                    outcomes.push((repo.to_owned(), result));
                    if source_failed {
                        break;
                    }
                }
                if outcomes.iter().all(|(_, result)| result.is_ok()) {
                    let cached: Vec<_> = outcomes
                        .iter()
                        .map(|(repo, result)| (repo, result.as_ref().unwrap()))
                        .collect();
                    if let Ok(bytes) = serde_json::to_vec(&cached) {
                        let _ = std::fs::write(cache_path, bytes);
                    }
                }
                outcomes
            }
        })
        .collect::<Vec<_>>();
    let total_families = requests.len();
    let mut completed = 0;
    let mut results = stream::iter(requests).buffer_unordered(8);
    while let Some(outcomes) = results.next().await {
        completed += 1;
        eprintln!("Completed {completed}/{total_families} checkpoint families");
        for (repo, result) in outcomes {
            match result {
                Ok((model, exclusions)) => {
                    if let Some(model) = model {
                        if let Some(existing) = models.get_mut(&model.repo_id) {
                            ensure!(
                                existing.revision == model.revision,
                                "Repository revision changed during refresh: {}",
                                model.repo_id
                            );
                            for variant in model.variants {
                                if !existing
                                    .variants
                                    .iter()
                                    .any(|v| v.variant.variant_id == variant.variant.variant_id)
                                {
                                    existing.variants.push(variant);
                                }
                            }
                            existing
                                .variants
                                .sort_by(|a, b| a.variant.variant_id.cmp(&b.variant.variant_id));
                        } else {
                            models.insert(model.repo_id.clone(), model);
                        }
                    }
                    report.exclusions.extend(exclusions);
                }
                Err(error) if transient(&error) => report.errors.push(format!("{repo}: {error:#}")),
                Err(error) => report.exclusions.push(Exclusion {
                    repo,
                    variant: None,
                    reason: error.to_string(),
                }),
            }
        }
        if !report.errors.is_empty() {
            break;
        }
    }
    let mut models: Vec<_> = models.into_values().collect();
    models.sort_by(|a, b| {
        quality::compare_quality(
            index.models.get(&a.quality.evaluated_model_id),
            index.models.get(&b.quality.evaluated_model_id),
        )
        .then_with(|| b.downloads.cmp(&a.downloads))
        .then_with(|| a.repo_id.cmp(&b.repo_id))
    });
    report.models = models.len();
    report.variants = models.iter().map(|model| model.variants.len()).sum();
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;
    ensure!(
        report.errors.is_empty(),
        "Catalogue refresh had {} transient errors; existing catalogue preserved. See {}",
        report.errors.len(),
        report_path.display()
    );
    let catalogue = Catalogue {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION,
        eredu_revision: eredu_revision().into(),
        generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        profile: Profile::default(),
        benchmarks: quality::BENCHMARKS.iter().map(|s| (*s).into()).collect(),
        quality_index_version: index.version.clone(),
        models,
    };
    validate(&catalogue)?;
    let temporary = path.with_extension("json.tmp");
    let mut bytes = serde_json::to_vec_pretty(&catalogue)?;
    bytes.push(b'\n');
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)?;
    eprintln!(
        "Wrote {} models / {} variants to {}",
        report.models,
        report.variants,
        path.display()
    );
    Ok(())
}

pub async fn explain(repo: &str) -> Result<()> {
    let hub = Hub::public(None)?;
    let index = quality::QualityIndex::fetch().await?;
    let result = inspect_repo(
        &hub,
        &index,
        repo,
        &mut BTreeSet::new(),
        &mut BTreeMap::new(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Reapply lineage policy to already inspected immutable artifacts without repeating forecasts.
/// This only prunes the catalogue; a full rebuild also discovers replacement candidates.
pub async fn revalidate_lineage(path: &Path, report_path: &Path) -> Result<()> {
    let mut catalogue: Catalogue = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(
        matches!(catalogue.policy_version, 3 | POLICY_VERSION),
        "Unsupported catalogue policy; rebuild it"
    );
    catalogue.policy_version = POLICY_VERSION;
    validate_inner(&catalogue, false)?;
    let hub = Hub::public(Some(report_path.with_extension("metadata")))?;
    let mut exclusions = Vec::new();
    let mut accepted = Vec::new();
    for mut model in catalogue.models {
        if model.quality.inherited {
            eprintln!("Verifying lineage {}", model.repo_id);
            // Use the immutable artifact revision, including its card declarations.
            let manifest = hub
                .json(
                    &format!("api/models/{}/revision/{}", model.repo_id, model.revision),
                    &[("blobs", "true")],
                )
                .await?;
            let parent = lineage::evaluated(
                &hub,
                &model.repo_id,
                &manifest,
                &model.quality.evaluated_model_id,
            )
            .await;
            let parent = match parent {
                Ok(parent) => parent,
                Err(error) if transient(&error) => return Err(error),
                Err(error) => {
                    exclusions.push(Exclusion {
                        repo: model.repo_id,
                        variant: None,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            let mut variants = Vec::new();
            for mut item in model.variants {
                match lineage::verify(
                    &hub,
                    &model.repo_id,
                    &model.revision,
                    &item.variant,
                    &parent,
                )
                .await
                {
                    Ok(proof) => {
                        item.lineage = Some(proof);
                        variants.push(item);
                    }
                    Err(error) if transient(&error) => return Err(error),
                    Err(error) => exclusions.push(Exclusion {
                        repo: model.repo_id.clone(),
                        variant: Some(item.variant.variant_id),
                        reason: error.to_string(),
                    }),
                }
            }
            model.variants = variants;
        }
        if !model.variants.is_empty() {
            accepted.push(model);
        }
    }
    catalogue.models = accepted;
    validate(&catalogue)?;
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(report_path, serde_json::to_vec_pretty(&exclusions)?)?;
    let temporary = path.with_extension("json.tmp");
    let mut bytes = serde_json::to_vec_pretty(&catalogue)?;
    bytes.push(b'\n');
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)?;
    eprintln!(
        "Lineage verified: {} models; {} exclusions in {}",
        catalogue.models.len(),
        exclusions.len(),
        report_path.display()
    );
    Ok(())
}
