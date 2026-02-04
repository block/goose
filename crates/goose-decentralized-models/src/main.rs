use anyhow::Result;
use clap::{Parser, Subcommand};

use goose_decentralized_models::config::{
    detect_public_ip, AdvertiseEndpoint, ModelConfig, NostrShareConfig,
};
use goose_decentralized_models::keys::KeyManager;
use goose_decentralized_models::publisher::{ModelDiscovery, ModelPublisher};

#[derive(Parser)]
#[command(name = "goose-decentralized-models")]
#[command(about = "Share and discover LLM models via Nostr")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize config and generate keys
    Init,
    /// Publish your models to Nostr
    Publish,
    /// Remove a published model
    Unpublish { model_name: String },
    /// List your published models
    List,
    /// Discover models from others
    Discover,
    /// Show your Nostr public key
    ShowKey,
    /// Discover a model and launch goose with it
    Run {
        /// Preferred model name (optional, uses first available if not specified)
        #[arg(long)]
        model: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init().await,
        Commands::Publish => publish().await,
        Commands::Unpublish { model_name } => unpublish(&model_name).await,
        Commands::List => list().await,
        Commands::Discover => discover().await,
        Commands::ShowKey => show_key().await,
        Commands::Run { model } => run_goose(model).await,
    }
}

async fn init() -> Result<()> {
    let config_path = NostrShareConfig::default_path()?;

    if config_path.exists() {
        println!("Config already exists at: {}", config_path.display());
        println!("Delete it first if you want to reinitialize.");
        return Ok(());
    }

    // Detect public IP
    let host = match detect_public_ip().await {
        Ok(ip) => {
            println!("Detected public IP: {}", ip);
            ip
        }
        Err(_) => {
            println!("Could not detect public IP, using placeholder");
            "YOUR_IP".to_string()
        }
    };

    // Fetch models from Ollama
    let models = fetch_ollama_models().await.unwrap_or_default();
    let model_configs: Vec<ModelConfig> = if models.is_empty() {
        println!("No Ollama models found, using placeholder");
        vec![ModelConfig {
            name: "qwen3:latest".to_string(),
            display_name: Some("Qwen 3".to_string()),
            description: Some("Local Qwen 3 model".to_string()),
            context_size: Some(32000),
        }]
    } else {
        println!("Found {} Ollama models:", models.len());
        for m in &models {
            println!("  - {}", m);
        }
        models
            .into_iter()
            .map(|name| ModelConfig {
                name: name.clone(),
                display_name: Some(name.replace(":latest", "").replace(':', " ")),
                description: None,
                context_size: Some(32000),
            })
            .collect()
    };

    let config = NostrShareConfig {
        private_key: None,
        relays: vec![
            "wss://relay.damus.io".to_string(),
            "wss://nos.lol".to_string(),
            "wss://relay.nostr.band".to_string(),
        ],
        models: model_configs,
        ollama_endpoint: "http://localhost:11434".to_string(),
        advertise_endpoint: AdvertiseEndpoint {
            host,
            port: 11434,
            https: false,
        },
        ttl_seconds: 3600,
    };

    config.save(&config_path)?;
    println!("Created config at: {}", config_path.display());

    // Generate keys
    let key_manager = KeyManager::load_default_or_generate()?;
    println!("Your Nostr public key (npub): {}", key_manager.npub());

    Ok(())
}

async fn fetch_ollama_models() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    let text = resp.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let models: Vec<String> = json
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

async fn publish() -> Result<()> {
    let config = NostrShareConfig::load_default()?;
    let key_manager = KeyManager::load_default_or_generate()?;

    println!("Publishing as: {}", key_manager.npub());
    println!("Connecting to {} relays...", config.relays.len());

    let publisher = ModelPublisher::new(key_manager.keys().clone(), config.relays.clone()).await?;
    publisher.connect().await;

    let endpoint = config.resolve_endpoint().await?;
    println!(
        "Advertising endpoint: {}://{}:{}",
        if endpoint.https { "https" } else { "http" },
        endpoint.host,
        endpoint.port
    );
    println!(
        "TTL: {} seconds ({} minutes)",
        config.ttl_seconds,
        config.ttl_seconds / 60
    );

    println!("Publishing {} models:", config.models.len());
    for model in &config.models {
        println!(
            "  - {} ({})",
            model.display_name.as_ref().unwrap_or(&model.name),
            model.name
        );
    }

    let ids = publisher
        .publish_all(&config.models, &endpoint, config.ttl_seconds)
        .await?;

    println!("Published {} events:", ids.len());
    for id in &ids {
        println!("  - {}", id.to_hex());
    }

    println!(
        "Done! Models will expire in {} minutes. Run 'publish' again to refresh.",
        config.ttl_seconds / 60
    );
    Ok(())
}

async fn unpublish(model_name: &str) -> Result<()> {
    let config = NostrShareConfig::load_default()?;
    let key_manager = KeyManager::load_default_or_generate()?;

    let publisher = ModelPublisher::new(key_manager.keys().clone(), config.relays.clone()).await?;
    publisher.connect().await;

    let events = publisher.list_own_models().await?;
    let mut found = false;

    for event in &events {
        for tag in event.tags.iter() {
            let tag_vec: Vec<&str> = tag.as_slice().iter().map(|s| s.as_str()).collect();
            if tag_vec.len() >= 2 && tag_vec[0] == "model" && tag_vec[1] == model_name {
                publisher.unpublish(&event.id).await?;
                println!("Unpublished: {}", model_name);
                found = true;
                break;
            }
        }
    }

    if !found {
        println!("Model not found: {}", model_name);
    }

    Ok(())
}

async fn list() -> Result<()> {
    let config = NostrShareConfig::load_default()?;
    let key_manager = KeyManager::load_default_or_generate()?;

    let publisher = ModelPublisher::new(key_manager.keys().clone(), config.relays.clone()).await?;
    publisher.connect().await;

    let events = publisher.list_own_models().await?;

    if events.is_empty() {
        println!("No published models found.");
        return Ok(());
    }

    println!("Your published models ({}):", events.len());
    for event in &events {
        let mut model_name = "unknown".to_string();
        let mut expiration: Option<u64> = None;

        for tag in event.tags.iter() {
            let tag_vec: Vec<&str> = tag.as_slice().iter().map(|s| s.as_str()).collect();
            if tag_vec.len() >= 2 {
                match tag_vec[0] {
                    "model" => model_name = tag_vec[1].to_string(),
                    "expiration" => expiration = tag_vec[1].parse().ok(),
                    _ => {}
                }
            }
        }

        let exp_str = if let Some(exp) = expiration {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if exp > now {
                format!("expires in {} min", (exp - now) / 60)
            } else {
                "expired".to_string()
            }
        } else {
            "no expiration".to_string()
        };

        println!("  - {} ({})", model_name, exp_str);
    }

    Ok(())
}

async fn discover() -> Result<()> {
    let config = NostrShareConfig::load_default()?;

    println!(
        "Discovering LLM models on {} relays...",
        config.relays.len()
    );

    let discovery = ModelDiscovery::new(config.relays).await?;
    discovery.connect().await;

    let models = discovery.discover().await?;

    if models.is_empty() {
        println!("No models found.");
        return Ok(());
    }

    println!("Discovered {} models:", models.len());
    for model in &models {
        println!();
        println!(
            "  Model: {}",
            model.display_name.as_ref().unwrap_or(&model.model_name)
        );
        println!("    Name: {}", model.model_name);
        println!("    Publisher: {}", model.publisher_npub);
        println!("    Endpoint: {}", model.endpoint);
        if let Some(ctx) = model.context_size {
            println!("    Context: {}", ctx);
        }
        if let Some(exp) = model.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if exp > now {
                println!("    Expires: in {} min", (exp - now) / 60);
            }
        }
    }

    Ok(())
}

async fn show_key() -> Result<()> {
    let key_manager = KeyManager::load_default_or_generate()?;
    println!("Public key (npub): {}", key_manager.npub());
    println!("Public key (hex):  {}", key_manager.public_key_hex());
    Ok(())
}

async fn run_goose(preferred_model: Option<String>) -> Result<()> {
    let relays = NostrShareConfig::load_default().map(|c| c.relays).ok();

    let model = goose_decentralized_models::discover_model(relays, preferred_model.as_deref())
        .await?
        .ok_or_else(|| anyhow::anyhow!("No models available"))?;

    let goose_path = which::which("goose")
        .or_else(|_| which::which("./target/debug/goose"))
        .or_else(|_| which::which("./target/release/goose"))
        .map_err(|_| anyhow::anyhow!("Could not find goose binary"))?;

    let status = std::process::Command::new(goose_path)
        .env("GOOSE_PROVIDER", "ollama")
        .env("GOOSE_MODEL", &model.model_name)
        .env("OLLAMA_HOST", &model.endpoint)
        .status()?;

    std::process::exit(status.code().unwrap_or(1));
}
