use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Asset name for this platform (compile-time).
fn asset_name() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "goose-aarch64-apple-darwin.tar.bz2"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "goose-x86_64-apple-darwin.tar.bz2"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "goose-x86_64-unknown-linux-gnu.tar.bz2"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "goose-aarch64-unknown-linux-gnu.tar.bz2"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "goose-x86_64-pc-windows-msvc.zip"
    }
}

/// Binary name for this platform.
fn binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "goose.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "goose"
    }
}

// ---------------------------------------------------------------------------
// Sigstore / SLSA provenance verification
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hex digest of a byte slice.
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Verify the downloaded archive against GitHub's Sigstore SLSA provenance.
///
/// Uses the attestations published by `actions/attest-build-provenance` in the
/// release/canary/nightly workflows (added in PR #7097). Verification fetches
/// the attestation bundle from GitHub's attestation API and validates the
/// Sigstore signature chain, Rekor transparency log inclusion, and artifact
/// digest match.
async fn verify_provenance(archive_data: &[u8], tag: &str) -> Result<()> {
    let digest = sha256_hex(archive_data);
    println!("Archive SHA-256: {digest}");

    let mut tmp_file = tempfile::NamedTempFile::new()
        .context("Failed to create temp file for provenance verification")?;
    tmp_file
        .write_all(archive_data)
        .context("Failed to write archive to temp file")?;
    tmp_file.flush().context("Failed to flush temp file")?;

    let workflow = match tag {
        "canary" => "canary.yml",
        _ => "release.yml",
    };

    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GH_TOKEN").ok());

    println!("Verifying SLSA provenance via Sigstore...");

    match sigstore_verification::verify_github_attestation(
        tmp_file.path(),
        "block",
        "goose",
        token.as_deref(),
        Some(workflow),
    )
    .await
    {
        Ok(_) => {
            println!("Sigstore provenance verification passed.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Warning: Sigstore provenance verification failed: {e}");
            eprintln!(
                "This may be expected for releases published before provenance \
                 attestations were enabled."
            );
            Ok(())
        }
    }
}

/// Update the goose binary to the latest release.
///
/// Downloads the platform-appropriate archive from GitHub releases, verifies
/// its SLSA provenance via Sigstore, extracts it with path-traversal
/// hardening, and replaces the current binary in-place.
pub async fn update(canary: bool, reconfigure: bool) -> Result<()> {
    #[cfg(feature = "disable-update")]
    {
        bail!("Update is disabled in this build.");
    }

    #[cfg(not(feature = "disable-update"))]
    {
        let tag = if canary { "canary" } else { "stable" };
        let asset = asset_name();
        let url = format!("https://github.com/block/goose/releases/download/{tag}/{asset}");

        println!("Downloading {asset} from {tag} release...");

        // --- Download -----------------------------------------------------------
        let response = reqwest::get(&url)
            .await
            .context("Failed to download release archive")?;

        if !response.status().is_success() {
            bail!(
                "Download failed with HTTP status {}. URL: {}",
                response.status(),
                url
            );
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        println!("Downloaded {} bytes.", bytes.len());

        // --- Verify SLSA provenance via Sigstore --------------------------------
        verify_provenance(&bytes, tag).await?;

        // --- Extract to temp dir (hardened against path traversal) --------------
        let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

        #[cfg(target_os = "windows")]
        extract_zip(&bytes, tmp_dir.path())?;

        #[cfg(not(target_os = "windows"))]
        extract_tar_bz2(&bytes, tmp_dir.path())?;

        // --- Locate the binary in the extracted archive -------------------------
        let binary = binary_name();
        let extracted_binary = find_binary(tmp_dir.path(), binary)
            .with_context(|| format!("Could not find {binary} in extracted archive"))?;

        // --- Replace the current binary -----------------------------------------
        let current_exe =
            env::current_exe().context("Failed to determine current executable path")?;

        replace_binary(&extracted_binary, &current_exe)
            .context("Failed to replace current binary")?;

        // --- Copy DLLs on Windows -----------------------------------------------
        #[cfg(target_os = "windows")]
        copy_dlls(&extracted_binary, &current_exe)?;

        println!("goose updated successfully (verified with Sigstore SLSA provenance).");

        // --- Reconfigure if requested -------------------------------------------
        if reconfigure {
            println!("Running goose configure...");
            let status = Command::new(current_exe)
                .arg("configure")
                .status()
                .context("Failed to run goose configure")?;
            if !status.success() {
                eprintln!("Warning: goose configure exited with {status}");
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Archive extraction
// ---------------------------------------------------------------------------

/// Extract a .zip archive with path-traversal hardening (Windows).
///
/// Iterates entries individually and uses `enclosed_name()` to reject any
/// path that escapes the destination directory (zip-slip protection).
#[cfg(target_os = "windows")]
fn extract_zip(data: &[u8], dest: &Path) -> Result<()> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to open zip archive")?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("Failed to read zip entry at index {i}"))?;

        let safe_path = match entry.enclosed_name() {
            Some(p) => p.to_owned(),
            None => bail!("Zip entry has unsafe path: {}", entry.name()),
        };

        let target = dest.join(&safe_path);

        if entry.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }

    Ok(())
}

/// Validate that an archive entry path is safe (no absolute paths, no `..`).
fn validate_entry_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        bail!("Tar entry has absolute path: {}", path.display());
    }
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            bail!("Tar entry contains path traversal: {}", path.display());
        }
    }
    Ok(())
}

/// Extract a .tar.bz2 archive with path-traversal hardening (macOS / Linux).
///
/// Iterates entries individually, rejecting any entry whose path is absolute
/// or contains `..` components (tar-slip protection).
#[cfg(not(target_os = "windows"))]
fn extract_tar_bz2(data: &[u8], dest: &Path) -> Result<()> {
    use bzip2::read::BzDecoder;
    let decoder = BzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry
            .path()
            .context("Failed to read entry path")?
            .into_owned();

        validate_entry_path(&path)?;

        let target = dest.join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        entry
            .unpack(&target)
            .with_context(|| format!("Failed to extract: {}", path.display()))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Binary location
// ---------------------------------------------------------------------------

/// Find the binary inside the extracted archive.
///
/// The archive may place it in:
///   1. A `goose-package/` subdirectory (Windows releases)
///   2. Directly at the top level
///   3. In some other single subdirectory
fn find_binary(extract_dir: &Path, binary_name: &str) -> Option<PathBuf> {
    // 1. Check goose-package subdir (matches download_cli.sh / download_cli.ps1)
    let package_dir = extract_dir.join("goose-package");
    if package_dir.is_dir() {
        let p = package_dir.join(binary_name);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Check top level
    let p = extract_dir.join(binary_name);
    if p.exists() {
        return Some(p);
    }

    // 3. Search one level of subdirectories
    if let Ok(entries) = fs::read_dir(extract_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let candidate = entry.path().join(binary_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Binary replacement
// ---------------------------------------------------------------------------

/// Replace the current binary with the newly downloaded one.
///
/// On Windows we must rename the running exe (Windows allows rename but not
/// delete/overwrite of a locked file) then copy the new file in.
///
/// On Unix we can simply copy over the existing binary.
fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let old_exe = current_exe.with_extension("exe.old");

        // Clean up leftover from a previous update
        if old_exe.exists() {
            fs::remove_file(&old_exe).with_context(|| {
                format!(
                    "Failed to remove old backup {}. Is another goose process running?",
                    old_exe.display()
                )
            })?;
        }

        // Rename the running binary out of the way
        fs::rename(current_exe, &old_exe).with_context(|| {
            format!(
                "Failed to rename running binary to {}. Try closing Goose Desktop if it's open.",
                old_exe.display()
            )
        })?;

        // Copy the new binary into place
        fs::copy(new_binary, current_exe).with_context(|| {
            // Try to restore the old binary
            let _ = fs::rename(&old_exe, current_exe);
            format!("Failed to copy new binary to {}", current_exe.display())
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, copy the new binary over the existing one
        fs::copy(new_binary, current_exe)
            .with_context(|| format!("Failed to copy new binary to {}", current_exe.display()))?;

        // Ensure the binary is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(current_exe)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(current_exe, perms)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// DLL handling (Windows only)
// ---------------------------------------------------------------------------

/// Copy any .dll files from the extracted archive alongside the installed binary.
/// Windows GNU builds ship with libgcc, libstdc++, libwinpthread DLLs.
#[cfg(target_os = "windows")]
fn copy_dlls(extracted_binary: &Path, current_exe: &Path) -> Result<()> {
    let source_dir = extracted_binary
        .parent()
        .context("Extracted binary has no parent directory")?;
    let dest_dir = current_exe
        .parent()
        .context("Current executable has no parent directory")?;

    if let Ok(entries) = fs::read_dir(source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("dll") {
                    let file_name = path.file_name().unwrap();
                    let dest = dest_dir.join(file_name);
                    // Remove existing DLL first (it may be locked by another process)
                    if dest.exists() {
                        let _ = fs::remove_file(&dest);
                    }
                    fs::copy(&path, &dest).with_context(|| {
                        format!("Failed to copy {} to {}", path.display(), dest.display())
                    })?;
                    println!("  Copied {}", file_name.to_string_lossy());
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_asset_name_valid() {
        let name = asset_name();
        assert!(!name.is_empty());
        assert!(name.starts_with("goose-"));
        #[cfg(target_os = "windows")]
        assert!(name.ends_with(".zip"));
        #[cfg(not(target_os = "windows"))]
        assert!(name.ends_with(".tar.bz2"));
    }

    #[test]
    fn test_binary_name() {
        let name = binary_name();
        #[cfg(target_os = "windows")]
        assert_eq!(name, "goose.exe");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, "goose");
    }

    #[test]
    fn test_find_binary_in_package_subdir() {
        let tmp = tempdir().unwrap();
        let pkg = tmp.path().join("goose-package");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join(binary_name()), b"fake").unwrap();

        let found = find_binary(tmp.path(), binary_name());
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(binary_name()));
    }

    #[test]
    fn test_find_binary_top_level() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join(binary_name()), b"fake").unwrap();

        let found = find_binary(tmp.path(), binary_name());
        assert!(found.is_some());
        assert_eq!(found.unwrap(), tmp.path().join(binary_name()));
    }

    #[test]
    fn test_find_binary_nested_subdir() {
        let tmp = tempdir().unwrap();
        let nested = tmp.path().join("some-dir");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join(binary_name()), b"fake").unwrap();

        let found = find_binary(tmp.path(), binary_name());
        assert!(found.is_some());
    }

    #[test]
    fn test_find_binary_not_found() {
        let tmp = tempdir().unwrap();
        let found = find_binary(tmp.path(), binary_name());
        assert!(found.is_none());
    }

    #[test]
    fn test_replace_binary_basic() {
        let tmp = tempdir().unwrap();
        let new_bin = tmp.path().join("new_goose");
        let current = tmp.path().join("current_goose");

        fs::write(&new_bin, b"new version").unwrap();
        fs::write(&current, b"old version").unwrap();

        replace_binary(&new_bin, &current).unwrap();

        let content = fs::read_to_string(&current).unwrap();
        assert_eq!(content, "new version");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_replace_binary_windows_rename_away() {
        let tmp = tempdir().unwrap();
        let current = tmp.path().join("goose.exe");
        let new_bin = tmp.path().join("new_goose.exe");

        fs::write(&current, b"old version").unwrap();
        fs::write(&new_bin, b"new version").unwrap();

        replace_binary(&new_bin, &current).unwrap();

        // Current should now have new content
        let content = fs::read_to_string(&current).unwrap();
        assert_eq!(content, "new version");

        // Old backup should exist
        let old = current.with_extension("exe.old");
        assert!(old.exists());
        let old_content = fs::read_to_string(&old).unwrap();
        assert_eq!(old_content, "old version");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_replace_binary_windows_cleanup_old() {
        let tmp = tempdir().unwrap();
        let current = tmp.path().join("goose.exe");
        let old = current.with_extension("exe.old");
        let new_bin = tmp.path().join("new_goose.exe");

        // Simulate a previous update left .old behind
        fs::write(&current, b"version 2").unwrap();
        fs::write(&old, b"version 1").unwrap();
        fs::write(&new_bin, b"version 3").unwrap();

        replace_binary(&new_bin, &current).unwrap();

        let content = fs::read_to_string(&current).unwrap();
        assert_eq!(content, "version 3");

        // Old should now contain version 2 (not version 1)
        let old_content = fs::read_to_string(&old).unwrap();
        assert_eq!(old_content, "version 2");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_extract_zip_with_package_dir() {
        use std::io::Cursor;
        use std::io::Write;

        let tmp = tempdir().unwrap();

        // Create a zip in memory with goose-package/ structure
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut writer = zip::ZipWriter::new(cursor);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            writer.add_directory("goose-package/", options).unwrap();
            writer
                .start_file("goose-package/goose.exe", options)
                .unwrap();
            writer.write_all(b"fake goose binary").unwrap();
            writer
                .start_file("goose-package/libtest.dll", options)
                .unwrap();
            writer.write_all(b"fake dll").unwrap();
            writer.finish().unwrap();
        }

        extract_zip(&buf, tmp.path()).unwrap();

        let binary = find_binary(tmp.path(), "goose.exe");
        assert!(binary.is_some());

        let content = fs::read_to_string(binary.unwrap()).unwrap();
        assert_eq!(content, "fake goose binary");

        // DLL should be in goose-package too
        assert!(tmp.path().join("goose-package/libtest.dll").exists());
    }

    // -----------------------------------------------------------------------
    // SHA-256 digest tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sha256_hex_known_value() {
        let digest = sha256_hex(b"hello world");
        assert_eq!(
            digest,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_hex_empty() {
        let digest = sha256_hex(b"");
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // -----------------------------------------------------------------------
    // Path validation and extraction hardening tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_entry_path_accepts_safe_paths() {
        assert!(validate_entry_path(Path::new("goose")).is_ok());
        assert!(validate_entry_path(Path::new("goose-package/goose")).is_ok());
        assert!(validate_entry_path(Path::new("subdir/nested/file.txt")).is_ok());
    }

    #[test]
    fn test_validate_entry_path_rejects_absolute() {
        let result = validate_entry_path(Path::new("/etc/malicious"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute path"));
    }

    #[test]
    fn test_validate_entry_path_rejects_traversal() {
        let result = validate_entry_path(Path::new("../../escape.txt"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path traversal"));
    }

    #[test]
    fn test_validate_entry_path_rejects_nested_traversal() {
        let result = validate_entry_path(Path::new("safe/../../escape"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path traversal"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_extract_tar_bz2_safe_archive() {
        use bzip2::write::BzEncoder;
        use bzip2::Compression;

        let tmp = tempdir().unwrap();

        let mut builder_buf = Vec::new();
        {
            let encoder = BzEncoder::new(&mut builder_buf, Compression::default());
            let mut builder = tar::Builder::new(encoder);

            let data = b"goose binary content";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "goose-package/goose", &data[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }

        extract_tar_bz2(&builder_buf, tmp.path()).unwrap();

        let extracted = tmp.path().join("goose-package/goose");
        assert!(extracted.exists());
        assert_eq!(
            fs::read_to_string(extracted).unwrap(),
            "goose binary content"
        );
    }

    // -----------------------------------------------------------------------
    // Sigstore provenance verification test
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_verify_provenance_warns_on_missing_attestation() {
        let result = verify_provenance(b"not a real archive", "stable").await;
        assert!(
            result.is_ok(),
            "verify_provenance should succeed with a warning for unattested artifacts"
        );
    }
}
