use super::*;

pub fn skill_source_entry(
    name: &str,
    description: &str,
    content: &str,
    dir: &Path,
    global: bool,
    properties: HashMap<String, serde_json::Value>,
) -> SourceEntry {
    SourceEntry {
        source_type: SourceType::Skill,
        name: name.to_string(),
        description: description.to_string(),
        content: content.to_string(),
        path: dir.to_string_lossy().to_string(),
        global,
        writable: true,
        supporting_files: Vec::new(),
        properties,
    }
}

pub fn builtin_skill_entry(mut source: SourceEntry) -> SourceEntry {
    source.source_type = SourceType::BuiltinSkill;
    source.path = format!("builtin://skills/{}", source.name);
    source.global = true;
    source.supporting_files.clear();
    source
}
