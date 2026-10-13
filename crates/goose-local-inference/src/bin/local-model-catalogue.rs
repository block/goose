#[cfg(feature = "hf-hub")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use goose_local_inference::recommendations::catalogue;
    let mut args = std::env::args().skip(1);
    let action = args.next().unwrap_or_else(|| "--check".into());
    #[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
    if action == "--inspect-metadata" {
        return catalogue::run_worker();
    }
    let path = std::path::PathBuf::from(
        args.next()
            .unwrap_or_else(|| catalogue::DEFAULT_PATH.into()),
    );
    match action.as_str() {
        "--check" => {
            let catalogue = serde_json::from_slice(&std::fs::read(&path)?)?;
            catalogue::validate(&catalogue)?;
            println!("Catalogue valid: {} models", catalogue.models.len());
        }
        #[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
        "--explain" => {
            catalogue::explain(
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid repo ID"))?,
            )
            .await?
        }
        #[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
        "--revalidate-lineage" => catalogue::revalidate_lineage(&path, &std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/local-model-lineage-report.json".into()))).await?,
        "--rebuild" => {
            #[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
            catalogue::rebuild(
                &path,
                &std::path::PathBuf::from(
                    args.next()
                        .unwrap_or_else(|| "target/local-model-catalogue-report.json".into()),
                ),
            )
            .await?;
            #[cfg(not(all(feature = "eredu", target_os = "macos", target_arch = "aarch64")))]
            anyhow::bail!("Rebuilding requires --features hf-hub,eredu on Apple silicon; no Metal device or weights are needed");
        }
        _ => anyhow::bail!(
            "Usage: local-model-catalogue --check [catalogue.json] | --rebuild [catalogue.json] [report.json] | --explain owner/model | --revalidate-lineage [catalogue.json] [report.json]"
        ),
    }
    Ok(())
}
#[cfg(not(feature = "hf-hub"))]
fn main() {
    eprintln!("Enable the hf-hub feature to use local-model-catalogue");
    std::process::exit(1);
}
