use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::PathBuf;
use etcetera::{choose_app_strategy, AppStrategy};
use crate::config::APP_STRATEGY;
use chrono::Utc;

pub fn calculate_recipe_hash(recipe_content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(recipe_content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn get_recipe_hashes_dir() -> Result<PathBuf> {
    let recipe_hashes_path = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .config_dir()
        .join("recipe_hashes");

    fs::create_dir_all(&recipe_hashes_path)?;
    Ok(recipe_hashes_path)
}

pub fn has_accepted_recipe_before(recipe_content: &str) -> Result<bool> {
    let hash = calculate_recipe_hash(recipe_content);
    let hash_file = get_recipe_hashes_dir()?.join(format!("{}.hash", hash));
    Ok(hash_file.exists())
}

pub fn record_recipe_hash(recipe_content: &str) -> Result<()> {
    let hash = calculate_recipe_hash(recipe_content);
    let hash_file = get_recipe_hashes_dir()?.join(format!("{}.hash", hash));

    let timestamp = Utc::now().to_rfc3339();
    fs::write(hash_file, timestamp)?;
    Ok(())
}
