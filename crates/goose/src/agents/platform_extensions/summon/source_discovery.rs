use super::*;

pub(super) fn parse_agent_content(content: &str, path: &Path) -> Option<SourceEntry> {
    let (metadata, body): (AgentMetadata, String) = match parse_frontmatter(content) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return None,
        Err(e) => {
            // Missing fields means this file has valid YAML but isn't an agent — skip silently.
            // Only warn on actual YAML syntax errors.
            if e.to_string().contains("missing field") {
                return None;
            }
            warn!("Failed to parse agent file {}: {}", path.display(), e);
            return None;
        }
    };

    let description = metadata.description.unwrap_or_else(|| {
        let model_info = metadata
            .model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        format!("Agent{}", model_info)
    });

    Some(SourceEntry {
        source_type: SourceType::Agent,
        name: metadata.name,
        description,
        content: body,
        path: path.to_string_lossy().into_owned(),
        global: false,
        writable: true,
        supporting_files: Vec::new(),
        properties: std::collections::HashMap::new(),
    })
}

pub(super) fn scan_recipes_from_dir(
    dir: &Path,
    kind: SourceType,
    sources: &mut Vec<SourceEntry>,
    seen: &mut std::collections::HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !RECIPE_FILE_EXTENSIONS.contains(&ext) {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() || seen.contains(&name) {
            continue;
        }

        match Recipe::from_file_path(&path) {
            Ok(recipe) => {
                seen.insert(name.clone());
                sources.push(SourceEntry {
                    source_type: kind,
                    name,
                    description: recipe.description.clone(),
                    content: recipe.instructions.clone().unwrap_or_default(),
                    path: path.to_string_lossy().into_owned(),
                    global: false,
                    writable: true,
                    supporting_files: Vec::new(),
                    properties: std::collections::HashMap::new(),
                });
            }
            Err(e) => {
                warn!("Failed to parse recipe {}: {}", path.display(), e);
            }
        }
    }
}

pub(super) fn scan_agents_from_dir(
    dir: &Path,
    sources: &mut Vec<SourceEntry>,
    seen: &mut std::collections::HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read agent file {}: {}", path.display(), e);
                continue;
            }
        };

        if let Some(source) = parse_agent_content(&content, &path) {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }
    }
}

pub fn discover_filesystem_sources(working_dir: &Path) -> Vec<SourceEntry> {
    let mut sources: Vec<SourceEntry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let home = dirs::home_dir();
    let config = Paths::config_dir();

    let local_recipe_dirs: Vec<PathBuf> = vec![
        working_dir.to_path_buf(),
        working_dir.join(".goose/recipes"),
        working_dir.join(".agents/recipes"),
    ];

    let global_recipe_dirs: Vec<PathBuf> = std::env::var("GOOSE_RECIPE_PATH")
        .ok()
        .into_iter()
        .flat_map(|p| {
            let sep = if cfg!(windows) { ';' } else { ':' };
            p.split(sep).map(PathBuf::from).collect::<Vec<_>>()
        })
        .chain(
            [
                home.as_ref().map(|h| h.join(".goose/recipes")),
                Some(config.join("recipes")),
                home.as_ref().map(|h| h.join(".agents/recipes")),
            ]
            .into_iter()
            .flatten(),
        )
        .collect();

    let local_agent_dirs: Vec<PathBuf> = vec![
        working_dir.join(".goose/agents"),
        working_dir.join(".claude/agents"),
        working_dir.join(".agents/agents"),
    ];

    let global_agent_dirs: Vec<PathBuf> = [
        home.as_ref().map(|h| h.join(".goose/agents")),
        home.as_ref().map(|h| h.join(".agents/agents")),
        Some(config.join("agents")),
        home.as_ref().map(|h| h.join(".claude/agents")),
    ]
    .into_iter()
    .flatten()
    .collect();

    for dir in local_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceType::Recipe, &mut sources, &mut seen);
    }

    for dir in local_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    for dir in global_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceType::Recipe, &mut sources, &mut seen);
    }

    for dir in global_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    sources
}

impl SummonClient {
    pub(super) async fn get_working_dir(&self, session_id: &str) -> PathBuf {
        self.context
            .session_manager
            .get_session(session_id, false)
            .await
            .ok()
            .map(|s| s.working_dir)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    pub(super) async fn get_sources(
        &self,
        session_id: &str,
        working_dir: &Path,
    ) -> Vec<SourceEntry> {
        let fs_sources = self.get_filesystem_sources(working_dir).await;

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut sources: Vec<SourceEntry> = Vec::new();

        self.add_subrecipes(session_id, &mut sources, &mut seen)
            .await;

        for source in fs_sources {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }

        sources.sort_by(|a, b| (&a.source_type, &a.name).cmp(&(&b.source_type, &b.name)));
        sources
    }

    async fn get_filesystem_sources(&self, working_dir: &Path) -> Vec<SourceEntry> {
        let mut cache = self.source_cache.lock().await;
        if let Some((cached_at, cached_dir, sources)) = cache.as_ref() {
            if cached_dir == working_dir && cached_at.elapsed() < Duration::from_secs(60) {
                return sources.clone();
            }
        }
        let sources = self.discover_filesystem_sources(working_dir);
        *cache = Some((Instant::now(), working_dir.to_path_buf(), sources.clone()));
        sources
    }

    pub(super) async fn resolve_source(
        &self,
        session_id: &str,
        name: &str,
        working_dir: &Path,
    ) -> Result<Option<SourceEntry>, String> {
        let sources = self.get_sources(session_id, working_dir).await;

        if let Some(mut source) = sources.iter().find(|s| s.name == name).cloned() {
            if source.source_type == SourceType::Subrecipe && source.content.is_empty() {
                source.content = self.load_subrecipe_content(session_id, &source.name).await;
            }
            return Ok(Some(source));
        }

        Ok(None)
    }

    async fn load_subrecipe_content(&self, session_id: &str, name: &str) -> String {
        let session = match self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
        {
            Ok(s) => s,
            Err(_) => return String::new(),
        };

        let sub_recipes = match session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref()) {
            Some(sr) => sr,
            None => return String::new(),
        };

        let sr = match sub_recipes.iter().find(|sr| sr.name == name) {
            Some(sr) => sr,
            None => return String::new(),
        };

        match load_local_recipe_file(&sr.path) {
            Ok(recipe_file) => match Recipe::from_content(&recipe_file.content) {
                Ok(recipe) => {
                    let mut content = recipe.instructions.unwrap_or_default();
                    if let Some(params) = &recipe.parameters {
                        if !params.is_empty() {
                            content.push_str("\n\n");
                            content.push_str(&Self::format_parameters(params));
                        }
                    }
                    content
                }
                Err(_) => recipe_file.content,
            },
            Err(_) => String::new(),
        }
    }

    pub(super) fn discover_filesystem_sources(&self, working_dir: &Path) -> Vec<SourceEntry> {
        discover_filesystem_sources(working_dir)
    }

    async fn add_subrecipes(
        &self,
        session_id: &str,
        sources: &mut Vec<SourceEntry>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let session = match self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
        {
            Ok(s) => s,
            Err(_) => return,
        };

        let sub_recipes = match session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref()) {
            Some(sr) => sr,
            None => return,
        };

        for sr in sub_recipes {
            if seen.contains(&sr.name) {
                continue;
            }
            seen.insert(sr.name.clone());

            let description = self.build_subrecipe_description(sr).await;

            sources.push(SourceEntry {
                source_type: SourceType::Subrecipe,
                name: sr.name.clone(),
                description,
                content: String::new(),
                path: sr.path.clone(),
                global: false,
                writable: true,
                supporting_files: Vec::new(),
                properties: std::collections::HashMap::new(),
            });
        }
    }

    async fn build_subrecipe_description(&self, sr: &crate::recipe::SubRecipe) -> String {
        if let Some(desc) = &sr.description {
            return desc.clone();
        }

        if let Ok(recipe_file) = load_local_recipe_file(&sr.path) {
            if let Ok(recipe) = Recipe::from_content(&recipe_file.content) {
                let mut desc = recipe.description.clone();

                if let Some(params) = &recipe.parameters {
                    if !params.is_empty() {
                        desc = format!("{}\n{}", desc, Self::format_parameters(params));
                    }
                }

                return desc;
            }
        }

        format!("Subrecipe from {}", sr.path)
    }

    pub(super) fn format_parameters(params: &[RecipeParameter]) -> String {
        let mut out = String::from("Parameters:");
        for p in params {
            let mut detail = format!("\n  - {} ({}, {})", p.key, p.input_type, p.requirement);
            if let Some(default) = &p.default {
                detail.push_str(&format!(", default: \"{}\"", default));
            }
            if let Some(options) = &p.options {
                if !options.is_empty() {
                    detail.push_str(&format!(", options: [{}]", options.join(", ")));
                }
            }
            detail.push_str(&format!(": {}", p.description));
            out.push_str(&detail);
        }
        out
    }
}
