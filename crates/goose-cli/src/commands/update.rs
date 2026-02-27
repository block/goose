use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
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
        "goose-x86_64-pc-windows-gnu.zip"
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

/// Compute the hex-encoded SHA-256 digest of `data`.
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Download `<asset>.sha256` from the same release and verify against the
/// archive bytes. Returns Ok(true) if verified, Ok(false) if no checksum
/// file was published (graceful skip), or Err on mismatch.
async fn verify_checksum(archive_bytes: &[u8], tag: &str, asset: &str) -> Result<bool> {
    let checksum_url =
        format!("https://github.com/block/goose/releases/download/{tag}/{asset}.sha256");

    let resp = reqwest::get(&checksum_url)
        .await
        .context("Failed to fetch checksum file")?;

    // 404 means the checksum file hasn't been published yet — skip gracefully.
    // Any other non-success status or transport error should abort the update.
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if !resp.status().is_success() {
        bail!(
            "Checksum download failed with HTTP status {}",
            resp.status()
        );
    }

    let body = resp
        .text()
        .await
        .context("Failed to read checksum response")?;

    // Format: "<hex_digest>  <filename>\n" or just "<hex_digest>\n"
    let expected = body.split_whitespace().next().unwrap_or("").to_lowercase();

    if expected.is_empty() {
        bail!(
            "Checksum file was fetched but contains no digest. \
             The .sha256 asset may be corrupted or truncated."
        );
    }

    let actual = sha256_hex(archive_bytes);

    if actual != expected {
        bail!(
            "SHA-256 checksum mismatch!\n  expected: {}\n  actual:   {}\n\
             The downloaded archive may have been tampered with.",
            expected,
            actual
        );
    }

    Ok(true)
}

/// Update the goose binary to the latest release.
///
/// Downloads the platform-appropriate archive from GitHub releases,
/// verifies its SHA-256 checksum when available, extracts it with
/// path-traversal hardening, and replaces the current binary in-place.
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

        // --- Checksum verification ----------------------------------------------
        let digest = sha256_hex(&bytes);
        println!("SHA-256: {digest}");

        match verify_checksum(&bytes, tag, asset).await {
            Ok(true) => println!("Checksum verified."),
            Ok(false) => {
                eprintln!(
                    "Warning: no checksum file found for this release. \
                     Skipping verification."
                );
            }
            Err(e) => return Err(e),
        }

        // --- Extract to temp dir ------------------------------------------------
        let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

        extract_archive(asset, &bytes, tmp_dir.path())?;

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

        println!("goose updated successfully!");

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
// Archive extraction (hardened against path traversal)
// ---------------------------------------------------------------------------

/// Dispatch extraction based on the archive file extension.
fn extract_archive(name: &str, data: &[u8], dest: &Path) -> Result<()> {
    if name.ends_with(".tar.bz2") {
        extract_tar_bz2(data, dest)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(data, dest)
    } else if name.ends_with(".zip") {
        extract_zip(data, dest)
    } else {
        bail!("Unsupported archive format: {name}")
    }
}

/// Validate that an archive entry path is safe (no path traversal).
///
/// Rejects absolute paths and any component that is "..".
fn validate_entry_path(entry_path: &Path, dest: &Path) -> Result<PathBuf> {
    if entry_path.is_absolute() {
        bail!(
            "Refusing to extract entry with absolute path: {}",
            entry_path.display()
        );
    }

    for component in entry_path.components() {
        if let std::path::Component::ParentDir = component {
            bail!(
                "Refusing to extract entry with path traversal (..): {}",
                entry_path.display()
            );
        }
    }

    let full_path = dest.join(entry_path);
    Ok(full_path)
}

/// Extract a .tar.bz2 archive with path-traversal hardening.
fn extract_tar_bz2(data: &[u8], dest: &Path) -> Result<()> {
    use bzip2::read::BzDecoder;
    let decoder = BzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    unpack_tar_entries(&mut archive, dest).context("Failed to extract tar.bz2 archive")
}

/// Extract a .tar.gz archive with path-traversal hardening.
fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    unpack_tar_entries(&mut archive, dest).context("Failed to extract tar.gz archive")
}

/// Iterate tar entries individually, validating each path before extraction.
fn unpack_tar_entries<R: Read>(archive: &mut tar::Archive<R>, dest: &Path) -> Result<()> {
    for entry_result in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry_result.context("Failed to read tar entry")?;
        let entry_path = entry
            .path()
            .context("Failed to read entry path")?
            .into_owned();
        let target = validate_entry_path(&entry_path, dest)?;

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        entry
            .unpack(&target)
            .with_context(|| format!("Failed to unpack entry {}", entry_path.display()))?;
    }
    Ok(())
}

/// Extract a .zip archive with path-traversal hardening.
fn extract_zip(data: &[u8], dest: &Path) -> Result<()> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to open zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Failed to read zip entry")?;

        let entry_path = match file.enclosed_name() {
            Some(p) => p.to_owned(),
            None => {
                bail!(
                    "Refusing to extract zip entry with unsafe path: {}",
                    file.name()
                );
            }
        };

        let target = dest.join(&entry_path);

        if file.is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("Failed to create directory {}", target.display()))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&target)
                .with_context(|| format!("Failed to create file {}", target.display()))?;
            std::io::copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to write {}", target.display()))?;
        }

        // Preserve unix permissions when available
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&target, fs::Permissions::from_mode(mode))?;
            }
        }
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
    // 1. goose-package subdir (matches download_cli.sh / download_cli.ps1)
    let package_dir = extract_dir.join("goose-package");
    if package_dir.is_dir() {
        let p = package_dir.join(binary_name);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Top level
    let p = extract_dir.join(binary_name);
    if p.exists() {
        return Some(p);
    }

    // 3. One level of subdirectories
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
/// On Windows the running exe is renamed aside first (Windows allows rename
/// but not overwrite of a locked file), then the new file is copied in.
///
/// On Unix we copy directly and restore the executable permission bits.
fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let old_exe = current_exe.with_extension("exe.old");

        if old_exe.exists() {
            fs::remove_file(&old_exe).with_context(|| {
                format!(
                    "Failed to remove old backup {}. Is another goose process running?",
                    old_exe.display()
                )
            })?;
        }

        fs::rename(current_exe, &old_exe).with_context(|| {
            format!(
                "Failed to rename running binary to {}. Try closing Goose Desktop if it's open.",
                old_exe.display()
            )
        })?;

        fs::copy(new_binary, current_exe).with_context(|| {
            let _ = fs::rename(&old_exe, current_exe);
            format!("Failed to copy new binary to {}", current_exe.display())
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Copy to a temp file in the same directory, then atomic-rename into
        // place. Writing directly to a running executable fails with ETXTBSY
        // on Linux/macOS.
        let dest_dir = current_exe
            .parent()
            .context("Current executable has no parent directory")?;
        let tmp_file = dest_dir.join(".goose-update.tmp");

        fs::copy(new_binary, &tmp_file)
            .with_context(|| format!("Failed to copy new binary to {}", tmp_file.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&tmp_file)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tmp_file, perms)?;
        }

        fs::rename(&tmp_file, current_exe).with_context(|| {
            let _ = fs::remove_file(&tmp_file);
            format!(
                "Failed to rename {} to {}",
                tmp_file.display(),
                current_exe.display()
            )
        })?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// DLL handling (Windows only)
// ---------------------------------------------------------------------------

/// Copy any .dll files from the extracted archive alongside the installed binary.
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
    fn test_sha256_hex() {
        let digest = sha256_hex(b"hello world");
        assert_eq!(
            digest,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_validate_entry_path_rejects_absolute() {
        let tmp = tempdir().unwrap();
        let result = validate_entry_path(Path::new("/etc/passwd"), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_entry_path_rejects_traversal() {
        let tmp = tempdir().unwrap();
        let result = validate_entry_path(Path::new("../../../etc/passwd"), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_entry_path_accepts_safe() {
        let tmp = tempdir().unwrap();
        let result = validate_entry_path(Path::new("goose-package/goose"), tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.path().join("goose-package/goose"));
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

    #[test]
    fn test_extract_tar_bz2_rejects_traversal() {
        use bzip2::write::BzEncoder;
        use bzip2::Compression;
        use std::io::Write;

        let tmp = tempdir().unwrap();

        // Manually construct a tar with a path-traversal entry by writing
        // a raw GNU tar header. The tar::Builder rejects ".." paths, so we
        // bypass it to simulate a malicious archive.
        let mut tar_buf = Vec::new();
        {
            let data = b"malicious";
            let path = b"../../../tmp/evil";

            // 512-byte tar header
            let mut header = [0u8; 512];
            header[..path.len()].copy_from_slice(path);
            // File mode (octal, ASCII) at offset 100
            header[100..108].copy_from_slice(b"0000644\0");
            // Owner/group uid/gid at offset 108/116
            header[108..116].copy_from_slice(b"0001000\0");
            header[116..124].copy_from_slice(b"0001000\0");
            // File size in octal at offset 124
            let size_str = format!("{:011o}\0", data.len());
            header[124..136].copy_from_slice(size_str.as_bytes());
            // Mtime at offset 136
            header[136..148].copy_from_slice(b"00000000000\0");
            // Typeflag '0' (regular file) at offset 156
            header[156] = b'0';
            // Compute checksum: sum of all bytes with checksum field as spaces
            header[148..156].copy_from_slice(b"        ");
            let cksum: u32 = header.iter().map(|&b| b as u32).sum();
            let cksum_str = format!("{:06o}\0 ", cksum);
            header[148..156].copy_from_slice(cksum_str.as_bytes());

            tar_buf.extend_from_slice(&header);
            tar_buf.extend_from_slice(data);
            // Pad to 512-byte boundary
            let padding = 512 - (data.len() % 512);
            if padding < 512 {
                tar_buf.extend(std::iter::repeat_n(0u8, padding));
            }
            // End-of-archive marker (two 512-byte zero blocks)
            tar_buf.extend(std::iter::repeat_n(0u8, 1024));
        }

        let mut bz2_buf = Vec::new();
        {
            let mut encoder = BzEncoder::new(&mut bz2_buf, Compression::fast());
            encoder.write_all(&tar_buf).unwrap();
            encoder.finish().unwrap();
        }

        let result = extract_tar_bz2(&bz2_buf, tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("path traversal") || err_msg.contains(".."),
            "Expected path traversal error, got: {err_msg}"
        );
    }

    #[test]
    fn test_extract_tar_gz_valid() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let tmp = tempdir().unwrap();

        // Build a valid tar.gz with a safe entry
        let mut tar_buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_buf);
            let data = b"binary content";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "goose", &data[..])
                .unwrap();
            builder.finish().unwrap();
        }

        let mut gz_buf = Vec::new();
        {
            let mut encoder = GzEncoder::new(&mut gz_buf, Compression::fast());
            std::io::Write::write_all(&mut encoder, &tar_buf).unwrap();
            encoder.finish().unwrap();
        }

        extract_tar_gz(&gz_buf, tmp.path()).unwrap();
        assert!(tmp.path().join("goose").exists());
        let content = fs::read(tmp.path().join("goose")).unwrap();
        assert_eq!(content, b"binary content");
    }

    #[test]
    fn test_extract_archive_dispatch() {
        let tmp = tempdir().unwrap();
        let result = extract_archive("goose.tar.xz", &[], tmp.path());
        assert!(result.is_err());
    }
}
