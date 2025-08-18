use std::{path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tokio::task::JoinHandle;
use tokio::sync::OnceCell;
use tracing::{info, warn, error, debug, trace, span, Level};
use anyhow::Result;

static STARTED: OnceCell<()> = OnceCell::const_new();

#[derive(Clone, Debug)]
pub struct AutoIndexConfig {
    pub root: PathBuf,
    pub output: PathBuf,
    pub enable_watch: bool,
    pub debounce: Duration,
    pub initial_delay: Duration,
    pub quiet: bool,
}

impl AutoIndexConfig {
    pub fn from_env(root: &Path) -> Option<Self> {
        if std::env::var("ALPHA_FEATURES").ok().as_deref() != Some("true") { return None; }
        // Opt-out variable
        if std::env::var("GOOSE_AUTO_INDEX").map(|v| v=="0" || v.to_lowercase()=="false").unwrap_or(false) { return None; }
        let enable_watch = std::env::var("GOOSE_AUTO_INDEX_WATCH").map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(false);
        let debounce_ms = std::env::var("GOOSE_AUTO_INDEX_DEBOUNCE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500u64);
        let initial_delay_ms = std::env::var("GOOSE_AUTO_INDEX_INITIAL_DELAY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500u64);
        Some(Self { root: root.to_path_buf(), output: root.join(".goose-repo-index.jsonl"), enable_watch, debounce: Duration::from_millis(debounce_ms), initial_delay: Duration::from_millis(initial_delay_ms), quiet: false })
    }
}

fn should_skip_repo(root: &Path) -> bool {
    // Heuristic: skip if no "src" or "lib" or very small (# source files < 5) to save cycles
    let mut source_like = 0usize;
    if let Ok(read) = std::fs::read_dir(root) {
        for entry in read.flatten().take(200) { // cheap scan
            let p = entry.path();
            if p.is_dir() { continue; }
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                match ext { "rs"|"py"|"js"|"ts"|"go"|"java"|"cs"|"cpp"|"cxx"|"cc"|"swift" => { source_like += 1; if source_like >= 5 { break; } }, _=>{} }
            }
        }
    }
    source_like < 5
}

pub async fn spawn_background_index() -> Option<JoinHandle<()>> {
    // Only run once per process
    if STARTED.set(()).is_err() { return None; }
    let root = match std::env::current_dir() { Ok(r) => r, Err(_) => return None };
    let cfg = match AutoIndexConfig::from_env(&root) { Some(c) => c, None => return None };
    if should_skip_repo(&cfg.root) { debug!(?cfg.root, "auto-index skipped: too few source files"); return None; }
    let lock_path = cfg.root.join(".goose-repo-index.lock");
    if lock_path.exists() { debug!(?lock_path, "auto-index lock exists, skipping"); return None; }
    if std::fs::write(&lock_path, b"indexing") .is_err() { return None; }
    info!(root=%cfg.root.display(), watch=%cfg.enable_watch, "Starting background repo index (initial delay {:?})", cfg.initial_delay);

    Some(tokio::spawn(async move {
        // Initial delay to avoid impacting startup latency
        tokio::time::sleep(cfg.initial_delay).await;
        if let Err(e) = run_index_once(&cfg).await { error!(error=?e, "background index failed"); }
        else { info!(root=%cfg.root.display(), "Background index complete"); }
        if cfg.enable_watch { if let Err(e) = watch_loop(&cfg).await { warn!(error=?e, "auto-index watch loop terminated"); } }
        let _ = std::fs::remove_file(&lock_path);
    }))
}

async fn run_index_once(cfg: &AutoIndexConfig) -> Result<()> {
    use goose::repo_index::RepoIndexOptions;
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut builder = RepoIndexOptions::builder().root(&cfg.root);
    let write_file = std::env::var("GOOSE_AUTO_INDEX_WRITE_FILE").map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(false);
    let mut maybe_file;
    if write_file { maybe_file = Some(File::create(&cfg.output)?); builder = builder.output_writer(maybe_file.as_mut().unwrap()); }
    else { builder = builder.output_null(); maybe_file = None; }
    let opts = builder.build();
    let build_start = std::time::Instant::now();
    let stats = goose::repo_index::index_repository(opts)?;
    let elapsed = build_start.elapsed();
    let duration_ms = stats.duration.as_millis();
    if !cfg.quiet { if write_file { eprintln!("(background) indexed {} files / {} entities (wrote {})", stats.files_indexed, stats.entities_indexed, cfg.output.display()); } else { eprintln!("(background) indexed {} files / {} entities (no file output)", stats.files_indexed, stats.entities_indexed); } }
    info!(counter.goose.repo.builds = 1, event="repo.index.build", root=%cfg.root.display(), background=true, files=stats.files_indexed, entities=stats.entities_indexed, duration_ms=elapsed.as_millis() as u64, wrote_file=write_file, trigger="background", "Background repository index build complete");
    // Write meta file for status command consumers
    let meta_path = cfg.root.join(".goose-repo-index.meta.json");
    let meta = serde_json::json!({
        "files_indexed": stats.files_indexed,
        "entities_indexed": stats.entities_indexed,
        "duration_ms": duration_ms,
        "wrote_file": write_file,
        "output_file": if write_file { Some(cfg.output.file_name().unwrap().to_string_lossy().to_string()) } else { None },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    });
    if let Err(e) = std::fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?) { warn!(error=?e, "failed to write meta file"); }
    Ok(())
}

async fn watch_loop(cfg: &AutoIndexConfig) -> Result<()> {
    use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
    use tokio::sync::mpsc;
    let (tx, mut rx) = mpsc::channel(64);
    let root = cfg.root.clone();
    let debounce = cfg.debounce;
    // Wrap notify watcher in blocking task -> channel
    let _blocking = std::thread::spawn(move || {
        let tx2 = tx.clone();
        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(|res| {
            if let Ok(event) = res { let _ = tx2.blocking_send(event); }
        }, notify::Config::default()).expect("watcher");
        let _ = watcher.watch(&root, RecursiveMode::Recursive);
        // park thread until process exit
        loop { std::thread::park(); }
    });

    let mut last_change: Option<Instant> = None;
    let mut pending = false;
    let mut ticker = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            maybe_evt = rx.recv() => {
                if let Some(evt) = maybe_evt { match evt.kind { EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => { last_change = Some(Instant::now()); pending = true; }, _=>{} } }
            }
            _ = ticker.tick() => {
                if pending { if let Some(ts) = last_change { if ts.elapsed() >= debounce { pending = false; if let Err(e) = run_index_once(cfg).await { warn!(error=?e, "background re-index failed"); } else { info!(counter.goose.repo.builds = 1, event="repo.index.build", root=%cfg.root.display(), background=true, trigger="watch", reason="watch", "Re-index complete (watch)"); } } } }
            }
        }
    }
}
