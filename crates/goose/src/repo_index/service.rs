use crate::repo_index::{RepoIndexOptions, RepoIndexStats};
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

// --- Enhanced import extraction helper (multi-language heuristics) ---
fn extract_import_modules(lang: &str, source: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    match lang {
        // Python: handle aliases, relative, multi-import
        "python" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("import ") {
                    let rest = &t[7..];
                    for part in rest.split(',') {
                        let mut token = part.trim();
                        if let Some(idx) = token.find(" as ") { token = &token[..idx]; }
                        token = token.split_whitespace().next().unwrap_or("");
                        if !token.is_empty() { set.insert(token.to_string()); }
                    }
                } else if t.starts_with("from ") {
                    if let Some(after_from) = t.strip_prefix("from ") {
                        if let Some((module, _imports)) = after_from.split_once(" import ") {
                            let mut mod_token = module.trim();
                            mod_token = mod_token.trim_start_matches('.'); // collapse relative dots
                            mod_token = mod_token.split('.').next().unwrap_or("");
                            if !mod_token.is_empty() { set.insert(mod_token.to_string()); }
                        }
                    }
                }
            }
        }
        // Local quoted includes only
        "cpp" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("#include \"") {
                    if let Some(start) = t.find('"') { if let Some(end_rel) = t[start+1..].find('"') { let name = &t[start+1..start+1+end_rel]; if !name.is_empty() { set.insert(normalize_module_basename(name)); } } }
                }
            }
        }
        "java" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("import ") {
                    let mut rest = &t[7..];
                    if rest.starts_with("static ") { rest = &rest[7..]; }
                    if let Some(semi) = rest.find(';') { rest = &rest[..semi]; }
                    let last = rest.rsplit('.').next().unwrap_or(rest).trim();
                    if !last.is_empty() && last != "*" { set.insert(last.to_string()); }
                }
            }
        }
        "javascript" | "typescript" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("import ") {
                    if let Some(idx) = t.find(" from ") {
                        let rest = &t[idx + 6..];
                        if let Some(m) = extract_quoted(rest) { set.insert(normalize_module_basename(&m)); }
                    } else if t.starts_with("import ") && (t.contains('"') || t.contains('\'')) {
                        if let Some(m) = extract_quoted(t) { set.insert(normalize_module_basename(&m)); }
                    }
                } else if t.contains("require(") {
                    if let Some(m) = between(t, "require(", ")") { let m2 = m.trim_matches(&['"', '\''] as &[_]); set.insert(normalize_module_basename(m2)); }
                }
            }
        }
        "go" => {
            let mut in_block = false;
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("import (") { in_block = true; continue; }
                if in_block {
                    if t.starts_with(')') { in_block = false; continue; }
                    if let Some(m) = extract_quoted(t) { set.insert(normalize_module_basename(&m)); }
                } else if t.starts_with("import ") {
                    if let Some(m) = extract_quoted(t) { set.insert(normalize_module_basename(&m)); }
                }
            }
        }
        "rust" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("mod ") {
                    let token = t[4..].split(|c: char| c == ';' || c == '{' || c.is_whitespace()).next().unwrap_or("");
                    if !token.is_empty() { set.insert(token.to_string()); }
                } else if t.starts_with("use ") {
                    let after = &t[4..];
                    let mut first = after.split(|c: char| c == ':' || c == ';' || c == '{' || c.is_whitespace()).next().unwrap_or("");
                    if ["crate","super","self"].contains(&first) {
                        let remainder = after.trim_start_matches(first).trim_start_matches(':').trim_start_matches(':');
                        first = remainder.split(|c: char| c == ':' || c == ';' || c == '{' || c.is_whitespace()).next().unwrap_or("");
                    }
                    if !first.is_empty() && first != "*" { set.insert(first.to_string()); }
                }
            }
        }
        "c_sharp" => {
            for line in source.lines() {
                let t = line.trim();
                if t.starts_with("using ") {
                    let after = &t[6..];
                    let head = after.split('=').next().unwrap_or(after);
                    let first = head.split(|c: char| c == '.' || c == ';' || c.is_whitespace()).next().unwrap_or("");
                    if !first.is_empty() { set.insert(first.to_string()); }
                }
            }
        }
        "swift" => {
            for line in source.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("@testable import ") { let tok = rest.split_whitespace().next().unwrap_or(""); if !tok.is_empty() { set.insert(tok.to_string()); } }
                else if let Some(rest) = t.strip_prefix("import ") { let tok = rest.split_whitespace().next().unwrap_or(""); if !tok.is_empty() { set.insert(tok.to_string()); } }
            }
        }
        _ => {}
    }
    set
}

fn extract_quoted(s: &str) -> Option<String> {
    let mut current = None;
    for (i, ch) in s.chars().enumerate() {
        if ch == '"' || ch == '\'' {
            if current.is_none() { current = Some((ch, i)); }
            else if let Some((qc, start)) = current { if qc == ch { return Some(s[start+1..i].to_string()); } }
        }
    }
    None
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let a = s.find(start)? + start.len();
    let rest = &s[a..];
    let b = rest.find(end)?;
    Some(&rest[..b])
}

fn normalize_module_basename(module: &str) -> String {
    let last = module.rsplit('/').next().unwrap_or(module);
    let stem = last.split('.').next().unwrap_or(last);
    stem.to_string()
}
use std::io::Write;

use ignore::WalkBuilder;
use tree_sitter::{Parser, Language};
use crate::repo_index::internal::{detect_language, lang_to_ts, extract_entities};

#[derive(Debug)]
pub struct FileRecord {
    pub id: u32,
    pub path: String,
    pub language: String,
    pub entities: Vec<u32>,
}

#[derive(Debug)]
pub struct StoredEntity {
    pub id: u32,
    pub file_id: u32,
    pub kind: EntityKind,
    pub name: String,
    pub parent: Option<String>,
    pub signature: String,
    pub start_line: u32,
    pub end_line: u32,
    pub calls: Vec<String>,
    pub doc: Option<String>,
    pub rank: f32, // placeholder for future PageRank
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind { File, Class, Function, Method, Other }

impl EntityKind {
    fn from_str(s: &str) -> Self {
        match s {
            "class" => EntityKind::Class,
            "function" => EntityKind::Function,
            "method" => EntityKind::Method,
            _ => EntityKind::Other,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self { EntityKind::File => "file", EntityKind::Class => "class", EntityKind::Function => "function", EntityKind::Method => "method", EntityKind::Other => "other" }
    }
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.as_str()) }
}

pub struct RepoIndexService {
    pub files: Vec<FileRecord>,
    pub entities: Vec<StoredEntity>,
    pub(crate) name_index: HashMap<String, Vec<u32>>, // lowercase name -> entity ids (crate visible)
    // Graph adjacency lists (indices reference entities vector)
    pub containment_children: Vec<Vec<u32>>, // entity id -> child entity ids
    pub containment_parent: Vec<Option<u32>>, // entity id -> optional parent entity id
    pub call_edges: Vec<Vec<u32>>, // entity id -> outgoing calls (resolved entity ids)
    pub reverse_call_edges: Vec<Vec<u32>>, // entity id -> incoming calls (callers)
    pub import_edges: Vec<Vec<u32>>, // placeholder until Step 4 (file-level imports -> file entity ids)
    pub file_entities: Vec<u32>, // mapping file index -> file entity id
    pub unresolved_imports: Vec<Vec<String>>, // per file index unresolved module basenames
    pub rank_weights: RankWeights, // configured (possibly env overridden) weights
}

#[derive(Clone, Copy, Debug)]
pub struct RankWeights {
    pub call: f32,
    pub import: f32,
    pub containment: f32,
    pub damping: f32,
    pub iterations: usize,
}

impl RankWeights {
    pub fn defaults() -> Self { Self { call: 1.0, import: 0.5, containment: 0.2, damping: 0.85, iterations: 20 } }
    pub fn from_env() -> Self {
        let mut w = Self::defaults();
        // Helper closure
        fn parse_f(name: &str) -> Option<f32> { std::env::var(name).ok().and_then(|v| v.parse::<f32>().ok()) }
        fn parse_usize(name: &str) -> Option<usize> { std::env::var(name).ok().and_then(|v| v.parse::<usize>().ok()) }
        if let Some(v) = parse_f("GOOSE_REPO_RANK_CALL_WEIGHT") { if v >= 0.0 { w.call = v; } }
        if let Some(v) = parse_f("GOOSE_REPO_RANK_IMPORT_WEIGHT") { if v >= 0.0 { w.import = v; } }
        if let Some(v) = parse_f("GOOSE_REPO_RANK_CONTAINMENT_WEIGHT") { if v >= 0.0 { w.containment = v; } }
        if let Some(v) = parse_f("GOOSE_REPO_RANK_DAMPING") { if (0.0..=1.0).contains(&v) { w.damping = v; } }
        if let Some(v) = parse_usize("GOOSE_REPO_RANK_ITERATIONS") { if v > 0 && v <= 200 { w.iterations = v; } }
        // If all edge weights zero, fall back to defaults to avoid division by zero
        if w.call == 0.0 && w.import == 0.0 && w.containment == 0.0 { return Self::defaults(); }
        w
    }
}

impl RepoIndexService {
    pub fn build(opts: RepoIndexOptions<'_>) -> Result<(Self, RepoIndexStats)> {
        let start = std::time::Instant::now();
        let root = opts.root;
        let walker = WalkBuilder::new(root).standard_filters(true).add_custom_ignore_filename(".gitignore").build();
        let mut files_map: HashMap<String, u32> = HashMap::new();
        let mut files: Vec<FileRecord> = Vec::new();
    let mut entities_store: Vec<StoredEntity> = Vec::new();
    let mut name_index: HashMap<String, Vec<u32>> = HashMap::new();
        let mut file_count = 0usize;
        let mut parser = Parser::new();
        // Temporary store of raw import module names per file index (not entity id yet)
        let mut file_import_modules: HashMap<u32, HashSet<String>> = HashMap::new();
        for dent in walker {
            let dent = match dent { Ok(d) => d, Err(_) => continue };
            let path = dent.path();
            if !path.is_file() { continue; }
            let lang: &str = match detect_language(path) { Some(l) => l, None => continue };
            if let Some(include) = &opts.include_langs { if !include.contains(lang) { continue; } }
            let language: Language = match lang_to_ts(lang) { Some(l) => l, None => continue };
            let src = match std::fs::read_to_string(path) { Ok(s) => s, Err(_) => continue };
            if parser.set_language(&language).is_err() {
                // Fallback: if language is one of our import-heuristic supported languages, still record file & imports
                if matches!(lang, "c_sharp" | "swift") {
                    let imports = extract_import_modules(lang, &src);
                    let file_path_str = path.display().to_string();
                    let file_id = *files_map.entry(file_path_str.clone()).or_insert_with(|| {
                        let id = files.len() as u32;
                        files.push(FileRecord { id, path: file_path_str.clone(), language: lang.to_string(), entities: Vec::new() });
                        id
                    });
                    if !imports.is_empty() { file_import_modules.entry(file_id).or_default().extend(imports); }
                }
                continue;
            }
            let tree = match parser.parse(&src, None) { Some(t) => t, None => continue };
            let mut entities_local = Vec::new();
            extract_entities(lang, &tree, &src, path, &mut entities_local);
            // Extract basic import/module references heuristically (language-specific)
            let imports = extract_import_modules(lang, &src);
            let file_path_str = path.display().to_string();
            let file_id = *files_map.entry(file_path_str.clone()).or_insert_with(|| {
                let id = files.len() as u32;
                files.push(FileRecord { id, path: file_path_str.clone(), language: lang.to_string(), entities: Vec::new() });
                id
            });
            if !imports.is_empty() { file_import_modules.entry(file_id).or_default().extend(imports); }
            for ent in entities_local {
                let id = entities_store.len() as u32;
                if let Some(fr) = files.get_mut(file_id as usize) { fr.entities.push(id); }
                name_index.entry(ent.name.to_lowercase()).or_default().push(id);
                entities_store.push(StoredEntity {
                    id,
                    file_id,
                    kind: EntityKind::from_str(ent.kind),
                    name: ent.name,
                    parent: ent.parent,
                    signature: ent.signature,
                    start_line: ent.start_line as u32,
                    end_line: ent.end_line as u32,
                    calls: ent.calls.unwrap_or_default(),
                    doc: ent.doc,
                    rank: 0.0,
                });
            }
            file_count += 1;
        }
        // Add file pseudo-entities (not added to name index to avoid symbol noise)
        let mut file_entities: Vec<u32> = Vec::with_capacity(files.len());
        for f in &files {
            let id = entities_store.len() as u32;
            file_entities.push(id);
            entities_store.push(StoredEntity {
                id,
                file_id: f.id,
                kind: EntityKind::File,
                name: std::path::Path::new(&f.path).file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(),
                parent: None,
                signature: String::new(),
                start_line: 0,
                end_line: 0,
                calls: Vec::new(),
                doc: None,
                rank: 0.0,
            });
        }

        // Initialize empty adjacency lists sized to entities length (after file entities appended)
        let entity_len = entities_store.len();
        let mut containment_children: Vec<Vec<u32>> = vec![Vec::new(); entity_len];
        let mut containment_parent: Vec<Option<u32>> = vec![None; entity_len];
        let mut call_edges: Vec<Vec<u32>> = vec![Vec::new(); entity_len];
        let mut reverse_call_edges: Vec<Vec<u32>> = vec![Vec::new(); entity_len];
    let mut import_edges: Vec<Vec<u32>> = vec![Vec::new(); entity_len];
        let mut unresolved_imports_per_file: Vec<Vec<String>> = vec![Vec::new(); files.len()];

        // Build a temporary map: (file_id, parent_name_lowercase) -> entity id for containment
        let mut scope_map: HashMap<(u32, String), u32> = HashMap::new();
        for e in &entities_store {
            scope_map.insert((e.file_id, e.name.to_lowercase()), e.id);
        }

        // Resolve containment relationships
        for e in &entities_store {
            if let Some(parent_name) = &e.parent {
                let key = (e.file_id, parent_name.to_lowercase());
                if let Some(parent_id) = scope_map.get(&key) {
                    containment_parent[e.id as usize] = Some(*parent_id);
                    containment_children[*parent_id as usize].push(e.id);
                }
            }
        }

        // Resolve call edges by name within same file first
        for e in &entities_store {
            if matches!(e.kind, EntityKind::File) { continue; }
            let mut resolved_local: HashSet<u32> = HashSet::new();
            for call_name in &e.calls {
                if let Some(callee_id) = scope_map.get(&(e.file_id, call_name.to_lowercase())) {
                    call_edges[e.id as usize].push(*callee_id);
                    reverse_call_edges[*callee_id as usize].push(e.id);
                    resolved_local.insert(*callee_id);
                }
            }
        }
        // Cross-file resolution: if a call name is globally unique, link it
        for e in &entities_store {
            if matches!(e.kind, EntityKind::File) { continue; }
            // gather already resolved callees to avoid duplicates
            let already: HashSet<u32> = call_edges[e.id as usize].iter().cloned().collect();
            for call_name in &e.calls {
                let key = call_name.to_lowercase();
                if let Some(ids) = name_index.get(&key) {
                    if ids.len() == 1 { // unambiguous
                        let target = ids[0];
                        if !already.contains(&target) && target != e.id { // avoid self-loop
                            call_edges[e.id as usize].push(target);
                            reverse_call_edges[target as usize].push(e.id);
                        }
                    }
                }
            }
        }

        // Resolve import edges: from file entity to target file entity based on basename/module name
        // Build map from lowercase basename (no extension) to file entity id (if unique)
        let mut basename_map: HashMap<String, u32> = HashMap::new();
        let mut basename_counts: HashMap<String, u32> = HashMap::new();
        for (idx, f) in files.iter().enumerate() {
            if let Some(stem) = std::path::Path::new(&f.path).file_stem().and_then(|s| s.to_str()) {
                let key = stem.to_lowercase();
                *basename_counts.entry(key.clone()).or_insert(0) += 1;
                basename_map.entry(key).or_insert(file_entities[idx]);
            }
        }
        for (fid, mods) in file_import_modules.into_iter() {
            let file_entity_id = file_entities[fid as usize];
            let mut added: HashSet<u32> = HashSet::new();
            for m in mods {
                let key = m.to_lowercase();
                if let Some(count) = basename_counts.get(&key) {
                    if *count == 1 {
                        if let Some(target_file_entity) = basename_map.get(&key) {
                            if added.insert(*target_file_entity) {
                                import_edges[file_entity_id as usize].push(*target_file_entity);
                            }
                            continue;
                        }
                    }
                    // ambiguous or missing mapping -> unresolved
                    unresolved_imports_per_file[fid as usize].push(m.clone());
                } else {
                    // no local match
                    unresolved_imports_per_file[fid as usize].push(m.clone());
                }
            }
        }

    let stats = RepoIndexStats { files_indexed: file_count, entities_indexed: entities_store.len(), duration: start.elapsed() };
    let rank_weights = RankWeights::from_env();
    let mut svc = Self { files, entities: entities_store, name_index, containment_children, containment_parent, call_edges, reverse_call_edges, import_edges, file_entities, unresolved_imports: unresolved_imports_per_file, rank_weights };
    // Compute initial PageRank (Step 5) with configured weights
    svc.compute_pagerank();
    Ok((svc, stats))
    }

    pub fn search_symbol_exact(&self, name: &str) -> Vec<&StoredEntity> {
        let key = name.to_lowercase();
        self.name_index.get(&key)
            .map(|ids| ids.iter().filter_map(|id| self.entities.get(*id as usize)).collect())
            .unwrap_or_default()
    }

    // Simple fuzzy search: compute lexical score (higher is better) then blend with PageRank.
    // Strategy: exact match score=1.0; prefix=0.8; substring=0.5; levenshtein within threshold (<=2) score=0.3 minus 0.1 per distance.
    // Final score = lexical_score * 0.6 + normalized_rank * 0.4 (normalization over all entities ranks).
    pub fn search_symbol_fuzzy_ranked(&self, query: &str, limit: usize) -> Vec<&StoredEntity> {
        let q_lower = query.to_lowercase();
        let mut min_rank = f32::MAX; let mut max_rank = f32::MIN;
        for e in &self.entities { if e.rank < min_rank { min_rank = e.rank; } if e.rank > max_rank { max_rank = e.rank; } }
        let rank_range = if (max_rank - min_rank).abs() < 1e-9 { 1.0 } else { max_rank - min_rank };
        fn levenshtein(a: &str, b: &str) -> usize { // small helper (O(len^2)) acceptable for modest entity counts
            let mut dp: Vec<usize> = (0..=b.len()).collect();
            for (i, ca) in a.chars().enumerate() {
                let mut prev = dp[0];
                dp[0] = i + 1;
                for (j, cb) in b.chars().enumerate() {
                    let temp = dp[j + 1];
                    dp[j + 1] = if ca == cb { prev } else { 1 + prev.min(dp[j]).min(dp[j + 1]) };
                    prev = temp;
                }
            }
            *dp.last().unwrap()
        }
        let mut scored: Vec<(f32, &StoredEntity)> = Vec::new();
        for e in &self.entities {
            // Skip file pseudo-entities for symbol searches
            if e.kind.as_str() == "file" { continue; }
            let name_lower = e.name.to_lowercase();
            let mut lex = 0.0f32;
            if name_lower == q_lower { lex = 1.0; }
            else if name_lower.starts_with(&q_lower) { lex = 0.8; }
            else if name_lower.contains(&q_lower) { lex = 0.5; }
            else {
                let dist = levenshtein(&name_lower, &q_lower);
                if dist <= 2 { lex = (0.3 - 0.1 * dist as f32).max(0.0); }
            }
            if lex > 0.0 { // candidate
                let norm_rank = (e.rank - min_rank) / rank_range;
                let final_score = lex * 0.6 + norm_rank * 0.4;
                scored.push((final_score, e));
            }
        }
        scored.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored.into_iter().map(|(_,e)| e).collect()
    }

    pub fn symbol_ids_exact(&self, name: &str) -> &[u32] {
        static EMPTY: [u32;0] = [];
        self.name_index.get(&name.to_lowercase()).map(|v| v.as_slice()).unwrap_or(&EMPTY)
    }

    pub fn children_of(&self, entity_id: u32) -> &[u32] {
        self.containment_children.get(entity_id as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn parent_of(&self, entity_id: u32) -> Option<u32> {
        self.containment_parent.get(entity_id as usize).and_then(|p| *p)
    }

    pub fn outgoing_calls(&self, entity_id: u32) -> &[u32] {
        self.call_edges.get(entity_id as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn incoming_calls(&self, entity_id: u32) -> &[u32] {
        self.reverse_call_edges.get(entity_id as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn imported_files(&self, file_entity_id: u32) -> &[u32] {
        self.import_edges.get(file_entity_id as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn unresolved_imports_for_file_index(&self, file_index: usize) -> &[String] {
        self.unresolved_imports.get(file_index).map(|v| v.as_slice()).unwrap_or(&[])
    }


// (Removed duplicate legacy import heuristic block; enhanced version defined above)
    pub fn export_jsonl<W: Write>(&self, mut w: W) -> Result<()> {
        for e in &self.entities {
            let json = serde_json::json!({
                "file": self.files[e.file_id as usize].path,
                "language": self.files[e.file_id as usize].language,
                "kind": e.kind.as_str(),
                "name": e.name,
                "parent": e.parent,
                "signature": e.signature,
                "start_line": e.start_line,
                "end_line": e.end_line,
                "calls": e.calls,
                "doc": e.doc,
                "rank": e.rank,
            });
            writeln!(w, "{}", json.to_string())?;
        }
        Ok(())
    }

    // Graph traversal helpers
    pub fn callees_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32> {
        self.bfs_depth(entity_id, depth, true)
    }
    pub fn callers_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32> {
        self.bfs_depth(entity_id, depth, false)
    }
    fn bfs_depth(&self, start: u32, depth: u32, forward: bool) -> Vec<u32> {
        if depth == 0 { return Vec::new(); }
        let mut visited: HashSet<u32> = HashSet::new();
        let mut out: Vec<u32> = Vec::new();
        let mut q: VecDeque<(u32, u32)> = VecDeque::new();
        q.push_back((start, 0));
        visited.insert(start);
        while let Some((node, d)) = q.pop_front() {
            if d == depth { continue; }
            let neigh = if forward { &self.call_edges } else { &self.reverse_call_edges };
            for &n in neigh.get(node as usize).unwrap_or(&Vec::new()) {
                if visited.insert(n) {
                    out.push(n);
                    q.push_back((n, d + 1));
                }
            }
        }
        out
    }

    // Weighted PageRank over multi-edge graph using configured RankWeights.
    pub fn compute_pagerank(&mut self) {
        let n = self.entities.len();
        if n == 0 { return; }
        let RankWeights { call: w_call, import: w_import, containment: w_contain, damping, iterations } = self.rank_weights;
        let init = 1.0f32 / n as f32;
        let mut rank = vec![init; n];
        let mut new_rank = vec![0.0f32; n];
        // Pre-build adjacency with normalized probabilities per source entity
        let mut outgoing: Vec<Vec<(u32, f32)>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut edges: Vec<(u32, f32)> = Vec::new();
            for &t in self.call_edges.get(i).unwrap_or(&Vec::new()) { edges.push((t, w_call)); }
            for &t in self.import_edges.get(i).unwrap_or(&Vec::new()) { edges.push((t, w_import)); }
            for &t in self.containment_children.get(i).unwrap_or(&Vec::new()) { edges.push((t, w_contain)); }
            if let Some(Some(p)) = self.containment_parent.get(i) { edges.push((*p, w_contain)); }
            let sum: f32 = edges.iter().map(|(_, w)| *w).sum();
            if sum > 0.0 { for e in edges.iter_mut() { e.1 /= sum; } }
            outgoing.push(edges);
        }
        let teleport = (1.0 - damping) / n as f32;
        for _ in 0..iterations {
            // reset
            new_rank.fill(0.0);
            let mut dangling_sum = 0.0f32;
            for i in 0..n {
                let r = rank[i];
                if outgoing[i].is_empty() { dangling_sum += r; continue; }
                for &(t, w) in &outgoing[i] { new_rank[t as usize] += r * w * damping; }
            }
            let dangling_contrib = if n > 0 { (dangling_sum * damping) / n as f32 } else { 0.0 };
            for i in 0..n { new_rank[i] += teleport + dangling_contrib; }
            std::mem::swap(&mut rank, &mut new_rank);
        }
        // Store back into entities
        for (i, r) in rank.into_iter().enumerate() { if let Some(ent) = self.entities.get_mut(i) { ent.rank = r; } }
    }
}
