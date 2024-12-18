use std::error::Error;
use std::fs;
use std::path::Path;

const BASE_DIR: &str = "../../tokenizer_files";
const MODELS: &[&str] = &[
    "Xenova/claude-tokenizer",
    "Xenova/gemma-2-tokenizer",
    "Xenova/gpt-4o",
    "Qwen/Qwen2.5-Coder-32B-Instruct",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create base directory
    fs::create_dir_all(BASE_DIR)?;
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", BASE_DIR);

    for model in MODELS {
        download_tokenizer(model).await?;
    }

    Ok(())
}

async fn download_tokenizer(repo_id: &str) -> Result<(), Box<dyn Error>> {
    let dir_name = repo_id.replace('/', "--");
    let download_dir = format!("{}/{}", BASE_DIR, dir_name);
    let file_url = format!(
        "https://huggingface.co/{}/resolve/main/tokenizer.json",
        repo_id
    );
    let file_path = format!("{}/tokenizer.json", download_dir);

    // Create directory if it doesn't exist
    fs::create_dir_all(&download_dir)?;

    // Check if file already exists
    if Path::new(&file_path).exists() {
        println!("Tokenizer for {} already exists, skipping...", repo_id);
        return Ok(());
    }

    println!("Downloading tokenizer for {}...", repo_id);

    // Download the file
    let response = reqwest::get(&file_url).await?;
    if !response.status().is_success() {
        return Err(format!("Failed to download tokenizer for {}", repo_id).into());
    }

    let content = response.bytes().await?;
    fs::write(&file_path, content)?;

    println!("Downloaded {} to {}", repo_id, file_path);
    Ok(())
}
