use anyhow::Result;
use std::path::Path;

pub fn status_repository(root_path: &str, json: bool) -> Result<()> {
    let root = Path::new(root_path);
    let meta_path = root.join(".goose-repo-index.meta.json");
    if !meta_path.exists() {
    println!("No index meta file found (path {}). Enable ALPHA_FEATURES and allow an auto-build by using repo tools or start background indexing.", meta_path.display());
        return Ok(());
    }
    let data = std::fs::read_to_string(&meta_path)?;
    if json {
        println!("{}", data);
        return Ok(());
    }
    let v: serde_json::Value = serde_json::from_str(&data)?;
    let files = v.get("files_indexed").and_then(|x| x.as_u64()).unwrap_or(0);
    let ents = v.get("entities_indexed").and_then(|x| x.as_u64()).unwrap_or(0);
    let dur = v.get("duration_ms").and_then(|x| x.as_u64()).unwrap_or(0);
    let wrote = v.get("wrote_file").and_then(|x| x.as_bool()).unwrap_or(false);
    let out_file = v.get("output_file").and_then(|x| x.as_str()).unwrap_or("-");
    println!("Indexed {} files / {} entities in {}ms{}", files, ents, dur, if wrote { format!(" (file: {})", out_file) } else { " (in-memory)".to_string() });
    Ok(())
}
