use super::*;

#[derive(Deserialize)]
pub struct MarkdownSourceFrontmatter {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

pub(super) fn projects_dir() -> PathBuf {
    Paths::data_dir().join("projects")
}

pub(super) fn project_file_path(slug: &str) -> PathBuf {
    projects_dir().join(format!("{slug}.md"))
}

pub(super) fn build_source_markdown(
    name: &str,
    description: &str,
    content: &str,
    properties: &HashMap<String, serde_json::Value>,
) -> Result<String, Error> {
    let mut frontmatter = serde_yaml::Mapping::new();
    frontmatter.insert(
        serde_yaml::Value::String("name".into()),
        serde_yaml::Value::String(name.into()),
    );
    frontmatter.insert(
        serde_yaml::Value::String("description".into()),
        serde_yaml::Value::String(description.into()),
    );
    for (key, value) in properties {
        if key == "name" || key == "description" {
            continue;
        }
        let value = serde_yaml::to_value(value).map_err(|e| {
            Error::internal_error().data(format!("Failed to serialize source property: {e}"))
        })?;
        frontmatter.insert(serde_yaml::Value::String(key.clone()), value);
    }
    let yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|e| Error::internal_error().data(format!("Failed to serialize source: {e}")))?;
    let mut md = format!("---\n{yaml}---\n");
    if !content.is_empty() {
        md.push('\n');
        md.push_str(content);
        md.push('\n');
    }
    Ok(md)
}

/// Returns (display_name, description, body, properties).
pub(super) fn parse_project_frontmatter(
    raw: &str,
) -> (String, String, String, HashMap<String, serde_json::Value>) {
    if !raw.trim_start().starts_with("---") {
        return (
            String::new(),
            String::new(),
            raw.to_string(),
            HashMap::new(),
        );
    }
    match parse_frontmatter::<MarkdownSourceFrontmatter>(raw) {
        Ok(Some((meta, body))) => (meta.name, meta.description, body, meta.properties),
        _ => (
            String::new(),
            String::new(),
            raw.to_string(),
            HashMap::new(),
        ),
    }
}

/// Validate a project slug. Same shape as a skill name (kebab-case, ASCII).
pub(super) fn validate_project_slug(slug: &str) -> Result<(), Error> {
    validate_skill_name(slug)
}

/// Read the `metadata:` field out of an existing SKILL.md, returning an
/// empty map if the file is missing, malformed, or carries no metadata.
pub(super) fn read_existing_skill_properties(
    skill_dir: &Path,
) -> HashMap<String, serde_json::Value> {
    let raw = match fs::read_to_string(skill_dir.join("SKILL.md")) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    match parse_frontmatter::<crate::skills::SkillFrontmatter>(&raw) {
        Ok(Some((meta, _))) => meta.metadata,
        _ => HashMap::new(),
    }
}

/// Read the properties bag out of an existing project file.
pub(super) fn read_existing_project_properties(file: &Path) -> HashMap<String, serde_json::Value> {
    let raw = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let (_, _, _, properties) = parse_project_frontmatter(&raw);
    properties
}

pub(super) fn project_entry_from_file(file: &Path) -> Option<SourceEntry> {
    let slug = file.file_stem().and_then(|s| s.to_str())?.to_string();
    if slug.is_empty() {
        return None;
    }
    let raw = fs::read_to_string(file).ok()?;
    let (title, description, content, mut properties) = parse_project_frontmatter(&raw);
    let display_name = if title.is_empty() {
        slug.clone()
    } else {
        title
    };
    if display_name != slug {
        // Preserve the user-facing display name so the frontend doesn't have
        // to special-case slug vs title.
        properties.insert(
            "title".into(),
            serde_json::Value::String(display_name.clone()),
        );
    }
    Some(SourceEntry {
        source_type: SourceType::Project,
        name: slug,
        description,
        content,
        path: file.to_string_lossy().into_owned(),
        global: true,
        writable: true,
        supporting_files: Vec::new(),
        properties,
    })
}

/// Read all projects from `<dataDir>/projects/`.
pub(super) fn read_project_dir() -> Result<Vec<SourceEntry>, Error> {
    let dir = projects_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&dir)
        .map_err(|e| Error::internal_error().data(format!("Failed to read projects dir: {e}")))?;

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Some(entry) = project_entry_from_file(&path) {
            out.push(entry);
        }
    }
    Ok(out)
}

/// Read a single project source by slug.
pub fn read_project(slug: &str) -> Result<SourceEntry, Error> {
    validate_project_slug(slug)?;
    let file = project_file_path(slug);
    if !file.exists() {
        return Err(Error::invalid_params().data(format!("Project \"{}\" not found", slug)));
    }
    project_entry_from_file(&file)
        .ok_or_else(|| Error::internal_error().data("Failed to read project file"))
}

/// Get the working directories configured for a project, if any.
/// Returns an empty Vec when the project doesn't exist or has none configured.
pub fn project_working_dirs(slug: &str) -> Vec<String> {
    let entry = match read_project(slug) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entry
        .properties
        .get("workingDirs")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default()
}

/// Validate that the given path is a project file we manage and the file
/// exists. Returns the canonical path on success.
pub(super) fn resolve_project_path(path: &str) -> Result<PathBuf, Error> {
    let canonical_path = Path::new(path).canonicalize().map_err(|_| {
        Error::invalid_params().data(format!("Project source \"{}\" not found", path))
    })?;
    let canonical_root = projects_dir()
        .canonicalize()
        .unwrap_or_else(|_| projects_dir());
    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::invalid_params().data(format!(
            "Path \"{}\" is not a project source",
            canonical_path.display()
        )));
    }
    if canonical_path.extension().and_then(|e| e.to_str()) != Some("md") {
        return Err(
            Error::invalid_params().data(format!("Path \"{}\" is not a markdown file", path))
        );
    }
    if !canonical_path.is_file() {
        return Err(Error::invalid_params().data(format!("Project source \"{}\" not found", path)));
    }
    Ok(canonical_path)
}
