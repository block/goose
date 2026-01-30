use std::path::Path;

const MAX_COMPLETIONS: usize = 15;

pub fn complete_path(partial: &str, cwd: &Path) -> Vec<(String, bool)> {
    let (dir_path, prefix) = if partial.contains('/') {
        let last_slash = partial.rfind('/').unwrap();
        let dir_part = &partial[..=last_slash];
        let file_part = &partial[last_slash + 1..];

        let resolved_dir = if dir_part.starts_with("~/") {
            dirs::home_dir()
                .map(|h| h.join(&dir_part[2..]))
                .unwrap_or_else(|| cwd.join(dir_part))
        } else if Path::new(dir_part).is_absolute() {
            std::path::PathBuf::from(dir_part)
        } else {
            cwd.join(dir_part)
        };

        (resolved_dir, file_part.to_lowercase())
    } else {
        (cwd.to_path_buf(), partial.to_lowercase())
    };

    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        return vec![];
    };

    let mut completions: Vec<(String, bool)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            !name.starts_with('.') && name.starts_with(&prefix)
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (name, is_dir)
        })
        .collect();

    // Sort: directories first, then alphabetically
    completions.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
    });

    completions.truncate(MAX_COMPLETIONS);
    completions
}

pub fn derive_job_id_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            s.to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect()
        })
        .unwrap_or_default()
}
