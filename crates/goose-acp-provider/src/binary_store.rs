use crate::paths;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tracing::info;

const BINARY_STORE_DIR: &str = "agent_binaries";

#[derive(Clone, Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

pub struct BinaryStore {
    root: PathBuf,
    http: Client,
}

impl BinaryStore {
    pub fn new() -> Result<Self> {
        let root = paths::data_dir().join(BINARY_STORE_DIR);
        let http = Client::builder()
            .user_agent("goose-acp-client")
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { root, http })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.http.get(url).send().await.context("request failed")?;
        let resp = resp.error_for_status().context("download failed")?;
        Ok(resp.bytes().await?.to_vec())
    }

    fn platform_asset_names(binary_name: &str, tag_name: &str) -> Result<(String, String)> {
        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            bail!("unsupported architecture");
        };
        let platform = if cfg!(target_os = "macos") {
            "apple-darwin"
        } else if cfg!(target_os = "windows") {
            "pc-windows-msvc"
        } else if cfg!(target_os = "linux") {
            "unknown-linux-gnu"
        } else {
            bail!("unsupported OS");
        };
        let ext = if cfg!(target_os = "windows") {
            "zip"
        } else {
            "tar.gz"
        };
        let version_stripped = tag_name.trim_start_matches('v');
        let asset_name = format!("{binary_name}-{version_stripped}-{arch}-{platform}.{ext}");
        let bin_name = if cfg!(target_os = "windows") {
            format!("{binary_name}.exe")
        } else {
            binary_name.to_string()
        };
        Ok((asset_name, bin_name))
    }

    pub async fn ensure_github_release_binary(
        &self,
        repo: &str,
        binary_name: &str,
    ) -> Result<PathBuf> {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("failed to fetch release")?;
        let resp = resp.error_for_status().context("bad status from GitHub")?;
        let release: GitHubRelease = resp.json().await.context("failed to parse release JSON")?;

        let (asset_name, bin_name) = Self::platform_asset_names(binary_name, &release.tag_name)?;
        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name == asset_name)
            .with_context(|| format!("asset {asset_name} not found"))?;

        let agent_dir = self.root.join(binary_name);
        tokio::fs::create_dir_all(&agent_dir)
            .await
            .context("failed to create agent dir")?;
        let version_dir = agent_dir.join(&release.tag_name);
        let bin_path = version_dir.join(&bin_name);

        if tokio::fs::metadata(&bin_path).await.is_ok() {
            return Ok(bin_path);
        }

        info!(
            "downloading {} {} from {}",
            binary_name, release.tag_name, asset.browser_download_url
        );

        let bytes = self.download(&asset.browser_download_url).await?;

        if let Some(expected) = asset.digest {
            let expected = expected
                .trim()
                .trim_start_matches("sha256:")
                .to_ascii_lowercase();
            let actual = format!("{:x}", Sha256::digest(&bytes));
            if actual != expected {
                bail!("SHA256 mismatch: expected {expected}, got {actual}");
            }
        }

        let temp_dir = tempfile::Builder::new()
            .prefix("extract-")
            .tempdir_in(&agent_dir)
            .context("failed to create temp dir")?;
        let dest = temp_dir.path();

        if asset.name.ends_with(".zip") {
            let cursor = Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(cursor).context("invalid zip")?;
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).context("bad zip entry")?;
                let entry_path = entry
                    .enclosed_name()
                    .ok_or_else(|| anyhow!("unsafe zip path"))?;
                let out_path = dest.join(entry_path);

                if entry.is_dir() {
                    std::fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(p) = out_path.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    let mut outfile = std::fs::File::create(&out_path)?;
                    std::io::copy(&mut entry, &mut outfile)?;

                    #[cfg(unix)]
                    if let Some(mode) = entry.unix_mode() {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
                    }
                }
            }
        } else {
            let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(dest).context("failed to unpack tar.gz")?;
        }

        let extracted_bin = dest.join(&bin_name);
        anyhow::ensure!(extracted_bin.is_file(), "extracted binary not found");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = extracted_bin.metadata()?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&extracted_bin, perms)?;
        }

        let persisted_temp = temp_dir.keep();
        tokio::fs::rename(&persisted_temp, &version_dir)
            .await
            .with_context(|| format!("failed to move to {}", version_dir.display()))?;

        Ok(version_dir.join(&bin_name))
    }
}
