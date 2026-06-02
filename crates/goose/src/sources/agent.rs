use super::*;

pub fn agent_base_dir(global: bool, project_dir: Option<&str>) -> Result<PathBuf, Error> {
    if global {
        Ok(Paths::agents_dir())
    } else {
        let project_dir = project_dir.ok_or_else(|| {
            Error::invalid_params().data("projectDir is required when global is false")
        })?;
        if project_dir.trim().is_empty() {
            return Err(
                Error::invalid_params().data("projectDir must not be empty when global is false")
            );
        }
        Ok(Path::new(project_dir).join(".agents").join("agents"))
    }
}

pub fn validate_agent_name(name: &str) -> Result<(), Error> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::invalid_params().data("Agent name must not be empty"));
    }
    if trimmed.len() > 80 {
        return Err(Error::invalid_params().data(format!(
            "Invalid agent name \"{}\". Names must be at most 80 characters.",
            name
        )));
    }
    if trimmed.chars().any(|ch| matches!(ch, '/' | '\\')) {
        return Err(Error::invalid_params().data(format!(
            "Invalid agent name \"{}\". Names must not contain path separators.",
            name
        )));
    }
    Ok(())
}

pub fn slugify_agent_name(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let mut collapsed = String::with_capacity(slug.len());
    let mut previous_hyphen = false;
    for ch in slug.chars() {
        if ch == '-' {
            if !previous_hyphen {
                collapsed.push('-');
            }
            previous_hyphen = true;
        } else {
            collapsed.push(ch);
            previous_hyphen = false;
        }
    }
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "agent".to_string()
    } else {
        trimmed
            .chars()
            .take(64)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string()
    }
}

pub fn parse_agent_frontmatter(raw: &str) -> Result<(MarkdownSourceFrontmatter, String), Error> {
    parse_frontmatter::<MarkdownSourceFrontmatter>(raw)
        .map_err(|e| Error::invalid_params().data(format!("Invalid agent frontmatter: {e}")))?
        .ok_or_else(|| Error::invalid_params().data("Agent file is missing frontmatter"))
}

pub fn agent_source_entry(path: &Path, global: bool, writable: bool) -> Result<SourceEntry, Error> {
    let raw = fs::read_to_string(path)
        .map_err(|e| Error::internal_error().data(format!("Failed to read agent file: {e}")))?;
    let (frontmatter, content) = parse_agent_frontmatter(&raw)?;
    Ok({
        SourceEntry {
            source_type: SourceType::Agent,
            name: frontmatter.name,
            description: frontmatter.description,
            content,
            path: path.to_string_lossy().to_string(),
            global,
            writable,
            supporting_files: Vec::new(),
            properties: frontmatter.properties,
        }
    })
}

pub fn read_existing_agent_properties(file: &Path) -> HashMap<String, serde_json::Value> {
    let raw = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    match parse_agent_frontmatter(&raw) {
        Ok((frontmatter, _)) => frontmatter.properties,
        Err(_) => HashMap::new(),
    }
}

pub fn canonicalize_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn is_under_root(path: &Path, root: &Path) -> bool {
    canonicalize_or_original(path).starts_with(canonicalize_or_original(root))
}

pub fn is_read_only_agent_file(path: &Path, additional_roots: &[SourceRoot]) -> bool {
    additional_roots
        .iter()
        .filter(|root| !root.writable)
        .any(|root| is_under_root(path, &root.path))
}

pub fn reject_read_only_agent_file(
    path: &Path,
    additional_roots: &[SourceRoot],
) -> Result<(), Error> {
    if is_read_only_agent_file(path, additional_roots) {
        return Err(Error::invalid_params().data("Source is read-only"));
    }
    Ok(())
}

pub fn is_global_agent_file(path: &Path) -> bool {
    let canonical_path = canonicalize_or_original(path);
    let mut global_roots = Vec::new();
    global_roots.push(Paths::agents_dir());
    if let Some(home) = dirs::home_dir() {
        global_roots.push(home.join(".agents").join("agents"));
        global_roots.push(home.join(".goose").join("agents"));
        global_roots.push(home.join(".claude").join("agents"));
    }
    global_roots.push(Paths::config_dir().join("agents"));

    global_roots
        .into_iter()
        .any(|root| canonical_path.starts_with(canonicalize_or_original(&root)))
}

pub fn resolve_agent_file_with_roots(
    path: &str,
    additional_roots: &[SourceRoot],
) -> Result<PathBuf, Error> {
    if path.is_empty() {
        return Err(Error::invalid_params().data("Source path must not be empty"));
    }

    let canonical_file = Path::new(path)
        .canonicalize()
        .map_err(|_| Error::invalid_params().data(format!("Source \"{}\" not found", path)))?;

    let parent_name = canonical_file
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    let grandparent_name = canonical_file
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    let in_agent_dir = parent_name == Some("agents")
        && matches!(
            grandparent_name,
            Some(".goose") | Some(".claude") | Some(".agents")
        );
    let in_additional_root = additional_roots
        .iter()
        .any(|root| is_under_root(&canonical_file, &root.path));

    if !canonical_file.is_file()
        || canonical_file.extension().and_then(|ext| ext.to_str()) != Some("md")
        || (!in_agent_dir && !is_global_agent_file(&canonical_file) && !in_additional_root)
    {
        return Err(Error::invalid_params().data(format!("Source \"{}\" not found", path)));
    }

    Ok(canonical_file)
}

pub fn list_agent_dirs(
    working_dir: Option<&Path>,
    additional_roots: &[SourceRoot],
) -> Vec<SourceRoot> {
    let mut dirs = Vec::new();
    if let Some(working_dir) = working_dir {
        dirs.push(SourceRoot {
            path: working_dir.join(".agents").join("agents"),
            writable: true,
        });
        dirs.push(SourceRoot {
            path: working_dir.join(".goose").join("agents"),
            writable: true,
        });
        dirs.push(SourceRoot {
            path: working_dir.join(".claude").join("agents"),
            writable: true,
        });
    }

    dirs.push(SourceRoot {
        path: Paths::agents_dir(),
        writable: true,
    });
    if let Some(home) = dirs::home_dir() {
        dirs.push(SourceRoot {
            path: home.join(".agents").join("agents"),
            writable: true,
        });
        dirs.push(SourceRoot {
            path: home.join(".goose").join("agents"),
            writable: true,
        });
        dirs.push(SourceRoot {
            path: home.join(".claude").join("agents"),
            writable: true,
        });
    }
    dirs.push(SourceRoot {
        path: Paths::config_dir().join("agents"),
        writable: true,
    });
    dirs.extend(additional_roots.iter().cloned());
    dirs
}

pub fn is_project_agent_file(path: &Path, working_dir: &Path) -> bool {
    [".agents", ".goose", ".claude"]
        .into_iter()
        .map(|dir| working_dir.join(dir).join("agents"))
        .any(|root| is_under_root(path, &root))
}

pub fn list_agent_sources(
    project_dir: Option<&str>,
    additional_roots: &[SourceRoot],
) -> Vec<SourceEntry> {
    let working_dir = project_dir
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let mut seen = std::collections::HashSet::new();
    let mut sources = Vec::new();

    for root in list_agent_dirs(working_dir.as_deref(), additional_roots) {
        let entries = match fs::read_dir(&root.path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let global = working_dir
                .as_deref()
                .is_none_or(|working_dir| !is_project_agent_file(&path, working_dir));
            match agent_source_entry(&path, global, root.writable) {
                Ok(source) => {
                    let key = source.name.to_lowercase();
                    if seen.insert(key) {
                        sources.push(source);
                    }
                }
                Err(err) => warn!("Skipping agent source {}: {:?}", path.display(), err),
            }
        }
    }

    sources
}

pub fn create_agent_source(
    name: &str,
    description: &str,
    content: &str,
    properties: HashMap<String, serde_json::Value>,
    global: bool,
    project_dir: Option<&str>,
) -> Result<SourceEntry, Error> {
    validate_agent_name(name)?;
    let base = agent_base_dir(global, project_dir)?;
    let slug = slugify_agent_name(name);
    let mut file_path = base.join(format!("{slug}.md"));
    if file_path.exists() {
        let mut counter = 2u32;
        loop {
            file_path = base.join(format!("{slug}-{counter}.md"));
            if !file_path.exists() {
                break;
            }
            counter += 1;
        }
    }

    fs::create_dir_all(&base).map_err(|e| {
        Error::internal_error().data(format!("Failed to create source directory: {e}"))
    })?;
    let md = build_source_markdown(name, description, content, &properties)?;
    fs::write(&file_path, md)
        .map_err(|e| Error::internal_error().data(format!("Failed to write agent file: {e}")))?;

    agent_source_entry(&file_path, global, true)
}

pub fn update_agent_source(
    path: &str,
    name: &str,
    description: &str,
    content: &str,
    properties: Option<HashMap<String, serde_json::Value>>,
    additional_roots: &[SourceRoot],
) -> Result<SourceEntry, Error> {
    validate_agent_name(name)?;
    let file_path = resolve_agent_file_with_roots(path, additional_roots)?;
    reject_read_only_agent_file(&file_path, additional_roots)?;
    let global = is_global_agent_file(&file_path);
    let resolved_properties = match properties {
        Some(p) => p,
        None => read_existing_agent_properties(&file_path),
    };
    let md = build_source_markdown(name, description, content, &resolved_properties)?;
    fs::write(&file_path, md)
        .map_err(|e| Error::internal_error().data(format!("Failed to write agent file: {e}")))?;

    agent_source_entry(&file_path, global, true)
}
