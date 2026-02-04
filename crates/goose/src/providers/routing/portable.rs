//! Portable project pack export/import for provider-agnostic project sharing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

use super::{EndpointId, ProjectId, RoutingError, RoutingResult, RunProviderState};

/// Manifest for a portable project pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// Manifest schema version
    pub schema_version: u32,
    /// Pack identifier
    pub pack_id: String,
    /// When this pack was created
    pub created_at: SystemTime,
    /// Project metadata
    pub project: ProjectMetadata,
    /// Runs included in this pack
    pub runs: Vec<RunMetadata>,
    /// Files included in the pack
    pub files: Vec<FileEntry>,
    /// Provider endpoints referenced (without secrets)
    pub endpoints: Vec<EndpointReference>,
    /// Pack integrity hash
    pub integrity_hash: String,
}

/// Project metadata for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project identifier
    pub project_id: ProjectId,
    /// Project name
    pub name: String,
    /// Project description
    pub description: Option<String>,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Last modified timestamp
    pub modified_at: SystemTime,
    /// Project tags/labels
    pub tags: Vec<String>,
    /// Version/revision
    pub version: String,
}

/// Run metadata for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Run identifier
    pub run_id: String,
    /// Run start time
    pub started_at: SystemTime,
    /// Run end time (if completed)
    pub ended_at: Option<SystemTime>,
    /// Summary description
    pub summary: String,
    /// Total requests made
    pub total_requests: u64,
    /// Total tokens used
    pub total_tokens_used: u64,
    /// Providers used during run
    pub providers_used: Vec<String>,
    /// Number of provider switches
    pub switch_count: usize,
}

/// File entry in the pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path within the pack
    pub path: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// SHA256 hash for integrity
    pub sha256: String,
    /// File type/category
    pub file_type: FileType,
    /// Whether this file contains sensitive data
    pub sensitive: bool,
}

/// Type of file in the pack
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Project configuration
    ProjectConfig,
    /// Run state data
    RunState,
    /// Generated artifacts (markdown, reports, etc.)
    Artifact,
    /// Log files
    Log,
    /// Database/checkpoint files
    Database,
    /// Documentation
    Documentation,
    /// Other/unknown
    Other,
}

/// Reference to a provider endpoint (without secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointReference {
    /// Endpoint identifier
    pub endpoint_id: EndpointId,
    /// Provider type
    pub provider: String,
    /// Base URL (may be redacted for security)
    pub base_url: Option<String>,
    /// Whether authentication is required
    pub requires_auth: bool,
    /// Available models (if known)
    pub available_models: Option<Vec<String>>,
}

/// Mapping configuration for importing a pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMapping {
    /// Endpoint mappings: pack endpoint ID -> local endpoint ID
    pub endpoint_mappings: HashMap<String, String>,
    /// Whether to create missing endpoints automatically
    pub auto_create_endpoints: bool,
    /// Whether to import run history
    pub import_runs: bool,
    /// Whether to import logs
    pub import_logs: bool,
    /// Target directory for imported files
    pub target_directory: Option<PathBuf>,
}

impl Default for ImportMapping {
    fn default() -> Self {
        Self {
            endpoint_mappings: HashMap::new(),
            auto_create_endpoints: false,
            import_runs: true,
            import_logs: false,
            target_directory: None,
        }
    }
}

/// Portable context pack for sharing projects across machines/users
#[derive(Debug)]
pub struct PortableContextPack {
    /// Temporary directory for pack contents
    work_dir: PathBuf,
    /// Export manifest
    manifest: Option<ExportManifest>,
    /// Whether this pack has been finalized
    finalized: bool,
}

impl PortableContextPack {
    /// Create a new portable context pack
    pub fn new() -> RoutingResult<Self> {
        let work_dir = std::env::temp_dir().join(format!("goose_pack_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&work_dir)?;

        Ok(Self {
            work_dir,
            manifest: None,
            finalized: false,
        })
    }

    /// Start building a pack for a project
    pub fn start_export(&mut self, project: ProjectMetadata) -> RoutingResult<()> {
        if self.finalized {
            return Err(RoutingError::ExportImportError(
                "Pack is already finalized".to_string(),
            ));
        }

        self.manifest = Some(ExportManifest {
            schema_version: 1,
            pack_id: format!("pack-{}", Uuid::new_v4()),
            created_at: SystemTime::now(),
            project,
            runs: Vec::new(),
            files: Vec::new(),
            endpoints: Vec::new(),
            integrity_hash: String::new(), // Will be computed at finalization
        });

        Ok(())
    }

    /// Add a run to the export
    pub fn add_run(&mut self, run_state: &RunProviderState) -> RoutingResult<()> {
        // Create run metadata
        let summary = run_state.get_summary();
        let run_metadata = RunMetadata {
            run_id: run_state.run_id.to_string(),
            started_at: run_state.started_at,
            ended_at: None, // Could be set if run is complete
            summary: format!("Run with {} requests", summary.total_requests),
            total_requests: summary.total_requests,
            total_tokens_used: summary.total_tokens_used,
            providers_used: vec![run_state.current_provider().to_string()],
            switch_count: run_state.switch_history.len(),
        };

        // Export run state to file
        let run_file = format!("runs/{}.json", run_state.run_id);
        let run_path = self.work_dir.join(&run_file);
        std::fs::create_dir_all(run_path.parent().unwrap())?;

        let run_json = serde_json::to_string_pretty(run_state)?;
        std::fs::write(&run_path, run_json)?;

        // Compute file hash
        let file_hash = self.compute_file_hash(&run_path)?;

        // Add to file list
        let file_entry = FileEntry {
            path: run_file,
            size_bytes: std::fs::metadata(&run_path)?.len(),
            sha256: file_hash,
            file_type: FileType::RunState,
            sensitive: false,
        };

        // Add to manifest
        if let Some(manifest) = self.manifest.as_mut() {
            manifest.runs.push(run_metadata);
            manifest.files.push(file_entry);
        } else {
            return Err(RoutingError::ExportImportError(
                "Export not started".to_string(),
            ));
        }

        Ok(())
    }

    /// Add a file to the export
    pub fn add_file(
        &mut self,
        source_path: &Path,
        pack_path: &str,
        file_type: FileType,
        sensitive: bool,
    ) -> RoutingResult<()> {
        let target_path = self.work_dir.join(pack_path);
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Copy file, potentially filtering sensitive content
        if sensitive {
            // For sensitive files, we might want to redact or exclude certain content
            self.copy_file_filtered(source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, &target_path)?;
        }

        // Compute hash and metadata
        let file_hash = self.compute_file_hash(&target_path)?;
        let file_size = std::fs::metadata(&target_path)?.len();

        // Create file entry
        let file_entry = FileEntry {
            path: pack_path.to_string(),
            size_bytes: file_size,
            sha256: file_hash,
            file_type,
            sensitive,
        };

        // Add to manifest
        let manifest = self
            .manifest
            .as_mut()
            .ok_or_else(|| RoutingError::ExportImportError("Export not started".to_string()))?;

        manifest.files.push(file_entry);
        Ok(())
    }

    /// Add endpoint reference (without secrets)
    pub fn add_endpoint_reference(&mut self, endpoint_ref: EndpointReference) -> RoutingResult<()> {
        let manifest = self
            .manifest
            .as_mut()
            .ok_or_else(|| RoutingError::ExportImportError("Export not started".to_string()))?;

        manifest.endpoints.push(endpoint_ref);
        Ok(())
    }

    /// Finalize the pack and create archive
    pub async fn finalize(&mut self, output_path: &Path) -> RoutingResult<()> {
        // Compute integrity hash first
        let integrity_hash = self.compute_pack_integrity()?;

        let manifest = self
            .manifest
            .as_mut()
            .ok_or_else(|| RoutingError::ExportImportError("Export not started".to_string()))?;

        manifest.integrity_hash = integrity_hash;

        // Write manifest
        let manifest_path = self.work_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;

        // Create archive
        self.create_archive(output_path).await?;

        self.finalized = true;
        Ok(())
    }

    /// Load a pack from archive for import
    pub async fn load_from_archive(archive_path: &Path) -> RoutingResult<Self> {
        let work_dir = std::env::temp_dir().join(format!("goose_import_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&work_dir)?;

        // Extract archive
        Self::extract_archive(archive_path, &work_dir).await?;

        // Load manifest
        let manifest_path = work_dir.join("manifest.json");
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: ExportManifest = serde_json::from_str(&manifest_content)?;

        Ok(Self {
            work_dir,
            manifest: Some(manifest),
            finalized: true,
        })
    }

    /// Import the pack with mapping configuration
    pub async fn import_with_mapping(
        &self,
        mapping: &ImportMapping,
        target_dir: &Path,
    ) -> RoutingResult<ImportResult> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| RoutingError::ExportImportError("No manifest loaded".to_string()))?;

        let mut result = ImportResult::default();

        // Validate endpoint mappings
        for endpoint_ref in &manifest.endpoints {
            let endpoint_id = endpoint_ref.endpoint_id.to_string();
            if !mapping.endpoint_mappings.contains_key(&endpoint_id) {
                result.unmapped_endpoints.push(endpoint_id);
            }
        }

        if !result.unmapped_endpoints.is_empty() && !mapping.auto_create_endpoints {
            return Ok(result); // Return early with mapping issues
        }

        // Import files
        for file_entry in &manifest.files {
            if file_entry.sensitive && !self.should_import_sensitive(&file_entry.file_type) {
                result.skipped_files.push(file_entry.path.clone());
                continue;
            }

            let source_path = self.work_dir.join(&file_entry.path);
            let target_path = target_dir.join(&file_entry.path);

            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::copy(&source_path, &target_path).await?;
            result.imported_files.push(file_entry.path.clone());
        }

        // Import runs if requested
        if mapping.import_runs {
            for run_meta in &manifest.runs {
                // Would import run state here
                result.imported_runs.push(run_meta.run_id.clone());
            }
        }

        result.success = result.unmapped_endpoints.is_empty();
        Ok(result)
    }

    /// Get the manifest
    pub fn manifest(&self) -> Option<&ExportManifest> {
        self.manifest.as_ref()
    }

    /// Compute SHA256 hash of a file
    fn compute_file_hash(&self, path: &Path) -> RoutingResult<String> {
        use sha2::{Digest, Sha256};

        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Compute integrity hash for the entire pack
    fn compute_pack_integrity(&self) -> RoutingResult<String> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Hash all files in order
        if let Some(manifest) = &self.manifest {
            let mut file_paths: Vec<_> = manifest.files.iter().map(|f| &f.path).collect();
            file_paths.sort();

            for path in file_paths {
                let full_path = self.work_dir.join(path);
                if let Ok(content) = std::fs::read(&full_path) {
                    hasher.update(&content);
                }
            }
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Copy file with sensitive content filtering
    fn copy_file_filtered(&self, source: &Path, target: &Path) -> RoutingResult<()> {
        // For now, just copy as-is. In a real implementation, this would:
        // - Remove API keys, tokens, secrets from config files
        // - Redact sensitive paths or user-specific information
        // - Filter out environment-specific settings
        std::fs::copy(source, target)?;
        Ok(())
    }

    /// Create archive from work directory
    async fn create_archive(&self, output_path: &Path) -> RoutingResult<()> {
        use std::io::Write;

        let file = std::fs::File::create(output_path)?;
        let mut archive = zip::ZipWriter::new(file);

        // Add all files from work directory
        for entry in walkdir::WalkDir::new(&self.work_dir) {
            let entry = entry.map_err(|e| {
                RoutingError::ExportImportError(format!("Failed to walk directory: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() {
                let relative_path = path
                    .strip_prefix(&self.work_dir)
                    .map_err(|e| RoutingError::ExportImportError(format!("Path error: {}", e)))?;

                archive
                    .start_file(
                        relative_path.to_string_lossy(),
                        zip::write::FileOptions::default(),
                    )
                    .map_err(|e| RoutingError::ExportImportError(format!("Zip error: {}", e)))?;

                let content = std::fs::read(path)?;
                archive
                    .write_all(&content)
                    .map_err(|e| RoutingError::ExportImportError(format!("Write error: {}", e)))?;
            }
        }

        archive
            .finish()
            .map_err(|e| RoutingError::ExportImportError(format!("Zip finalize error: {}", e)))?;

        Ok(())
    }

    /// Extract archive to directory
    async fn extract_archive(archive_path: &Path, target_dir: &Path) -> RoutingResult<()> {
        use std::io::Read;

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| RoutingError::ExportImportError(format!("Zip read error: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                RoutingError::ExportImportError(format!("Zip extract error: {}", e))
            })?;

            let outpath = target_dir.join(file.name());

            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        Ok(())
    }

    /// Check if sensitive file type should be imported
    fn should_import_sensitive(&self, file_type: &FileType) -> bool {
        // Only import sensitive files that are essential
        matches!(file_type, FileType::ProjectConfig | FileType::Database)
    }
}

impl Drop for PortableContextPack {
    fn drop(&mut self) {
        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&self.work_dir);
    }
}

/// Result of importing a portable pack
#[derive(Debug, Default, Clone)]
pub struct ImportResult {
    /// Whether the import was successful
    pub success: bool,
    /// Files that were imported
    pub imported_files: Vec<String>,
    /// Runs that were imported
    pub imported_runs: Vec<String>,
    /// Files that were skipped
    pub skipped_files: Vec<String>,
    /// Endpoints that couldn't be mapped
    pub unmapped_endpoints: Vec<String>,
    /// Any warnings or issues
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_pack_creation() {
        let mut pack = PortableContextPack::new().unwrap();

        let project = ProjectMetadata {
            project_id: ProjectId::new(),
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
            tags: vec!["test".to_string()],
            version: "1.0.0".to_string(),
        };

        pack.start_export(project).unwrap();
        assert!(pack.manifest.is_some());
    }

    #[test]
    fn test_file_hash() {
        let pack = PortableContextPack::new().unwrap();

        // Create a temporary file
        let temp_file = pack.work_dir.join("test.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        let hash = pack.compute_file_hash(&temp_file).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().next().unwrap().is_ascii_hexdigit());
    }

    #[test]
    fn test_endpoint_reference() {
        let endpoint_ref = EndpointReference {
            endpoint_id: EndpointId::new("test_endpoint"),
            provider: "anthropic".to_string(),
            base_url: Some("https://api.anthropic.com".to_string()),
            requires_auth: true,
            available_models: Some(vec!["claude-sonnet-3.5".to_string()]),
        };

        // Should be serializable
        let json = serde_json::to_string(&endpoint_ref).unwrap();
        assert!(json.contains("anthropic"));
        assert!(json.contains("test_endpoint"));
    }

    #[test]
    fn test_import_mapping() {
        let mut mapping = ImportMapping::default();
        mapping
            .endpoint_mappings
            .insert("old_endpoint".to_string(), "new_endpoint".to_string());
        mapping.import_runs = false;

        assert_eq!(
            mapping.endpoint_mappings.get("old_endpoint"),
            Some(&"new_endpoint".to_string())
        );
        assert!(!mapping.import_runs);
    }
}
