use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use super::parser::{Call, FileAnalysis, Symbol};

/// (file_path, symbol_name, definition_line) — line disambiguates same-name
/// functions in the same file (e.g. two `process()` in different impl blocks).
type NodeKey = (PathBuf, String, usize);

#[derive(Clone)]
pub struct ChainLink {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
}

struct Node {
    file: PathBuf,
    name: String,
    line: usize,
}

pub struct CallGraph {
    nodes: HashMap<NodeKey, Node>,
    // callee_key → set of caller_keys
    incoming: HashMap<NodeKey, HashSet<NodeKey>>,
    // caller_key → set of callee_keys
    outgoing: HashMap<NodeKey, HashSet<NodeKey>>,
}

impl CallGraph {
    pub fn build(analyses: &[FileAnalysis]) -> Self {
        let mut nodes: HashMap<NodeKey, Node> = HashMap::new();
        let mut incoming: HashMap<NodeKey, HashSet<NodeKey>> = HashMap::new();
        let mut outgoing: HashMap<NodeKey, HashSet<NodeKey>> = HashMap::new();

        // Register all symbols (functions + classes) as nodes
        for a in analyses {
            let register = |sym: &Symbol, nodes: &mut HashMap<NodeKey, Node>| {
                let key = (a.path.clone(), sym.name.clone(), sym.line);
                nodes.entry(key).or_insert_with(|| Node {
                    file: a.path.clone(),
                    name: sym.name.clone(),
                    line: sym.line,
                });
            };
            for f in &a.functions {
                register(f, &mut nodes);
            }
            for c in &a.classes {
                register(c, &mut nodes);
            }
        }

        // Build a name → keys index for resolving cross-file calls
        let mut name_index: HashMap<&str, Vec<NodeKey>> = HashMap::new();
        for key in nodes.keys() {
            name_index.entry(&key.1).or_default().push(key.clone());
        }

        // Build (path, name) → sorted definition lines for caller resolution.
        // When a Call says caller="process" at line 50, we pick the definition
        // of "process" whose line is the largest value ≤ 50 (nearest enclosing).
        let mut def_lines: HashMap<(&PathBuf, &str), Vec<usize>> = HashMap::new();
        for key in nodes.keys() {
            def_lines.entry((&key.0, &key.1)).or_default().push(key.2);
        }
        for lines in def_lines.values_mut() {
            lines.sort_unstable();
        }

        // Build path → language index to prevent cross-language false positives
        let lang_index: HashMap<&PathBuf, &str> =
            analyses.iter().map(|a| (&a.path, a.language)).collect();

        // Register edges from calls
        for a in analyses {
            for call in &a.calls {
                // Resolve caller: find the definition whose line is nearest-before the call site
                let caller_key = resolve_caller_key(a, call, &def_lines);
                // Resolve callee: same-file first, then cross-file (same language only)
                let callee_keys = resolve_callee(a, call, &name_index, &lang_index);
                if let Some(ck) = caller_key {
                    for callee_key in callee_keys {
                        incoming
                            .entry(callee_key.clone())
                            .or_default()
                            .insert(ck.clone());
                        outgoing.entry(ck.clone()).or_default().insert(callee_key);
                    }
                }
            }
        }

        Self {
            nodes,
            incoming,
            outgoing,
        }
    }

    pub fn definitions(&self, symbol: &str) -> Vec<ChainLink> {
        self.nodes
            .values()
            .filter(|n| n.name == symbol)
            .map(|n| ChainLink {
                file: n.file.clone(),
                name: n.name.clone(),
                line: n.line,
            })
            .collect()
    }

    pub fn incoming(&self, symbol: &str, depth: u32) -> Vec<Vec<ChainLink>> {
        let starts: Vec<NodeKey> = self
            .nodes
            .keys()
            .filter(|k| k.1 == symbol)
            .cloned()
            .collect();
        self.bfs_chains(&starts, depth, &self.incoming)
    }

    pub fn outgoing(&self, symbol: &str, depth: u32) -> Vec<Vec<ChainLink>> {
        let starts: Vec<NodeKey> = self
            .nodes
            .keys()
            .filter(|k| k.1 == symbol)
            .cloned()
            .collect();
        self.bfs_chains(&starts, depth, &self.outgoing)
    }

    fn bfs_chains(
        &self,
        starts: &[NodeKey],
        depth: u32,
        edges: &HashMap<NodeKey, HashSet<NodeKey>>,
    ) -> Vec<Vec<ChainLink>> {
        if depth == 0 {
            return vec![];
        }

        let mut chains = Vec::new();
        let mut queue: VecDeque<(Vec<NodeKey>, u32)> = VecDeque::new();

        for start in starts {
            if let Some(neighbors) = edges.get(start) {
                for neighbor in neighbors {
                    queue.push_back((vec![start.clone(), neighbor.clone()], 1));
                }
            }
        }

        while let Some((path, d)) = queue.pop_front() {
            let Some(tip) = path.last() else { continue };

            if d >= depth {
                chains.push(self.to_chain_links(&path));
                continue;
            }

            // Cycle detection: don't revisit nodes already in this path
            let visited: HashSet<&NodeKey> = path.iter().collect();

            match edges.get(tip) {
                Some(neighbors) => {
                    let mut extended = false;
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            let mut new_path = path.clone();
                            new_path.push(neighbor.clone());
                            queue.push_back((new_path, d + 1));
                            extended = true;
                        }
                    }
                    if !extended {
                        chains.push(self.to_chain_links(&path));
                    }
                }
                None => chains.push(self.to_chain_links(&path)),
            }
        }

        chains
    }

    fn to_chain_links(&self, path: &[NodeKey]) -> Vec<ChainLink> {
        path.iter()
            .map(|key| {
                let node = self.nodes.get(key);
                ChainLink {
                    file: key.0.clone(),
                    name: key.1.clone(),
                    line: node.map_or(0, |n| n.line),
                }
            })
            .collect()
    }
}

/// Given a call, find the NodeKey for the caller function. Uses the call's line
/// number to disambiguate when multiple functions share the same name in a file:
/// picks the definition whose line is the largest value ≤ call.line.
fn resolve_caller_key(
    analysis: &FileAnalysis,
    call: &Call,
    def_lines: &HashMap<(&PathBuf, &str), Vec<usize>>,
) -> Option<NodeKey> {
    let caller_name = &call.caller;
    if let Some(lines) = def_lines.get(&(&analysis.path, caller_name.as_str())) {
        // Binary search for the largest definition line ≤ call.line
        let line = match lines.binary_search(&call.line) {
            Ok(idx) => lines[idx],
            Err(0) => return None, // call is before any definition — shouldn't happen
            Err(idx) => lines[idx - 1],
        };
        Some((analysis.path.clone(), caller_name.clone(), line))
    } else {
        None
    }
}

fn resolve_callee(
    analysis: &FileAnalysis,
    call: &Call,
    name_index: &HashMap<&str, Vec<NodeKey>>,
    lang_index: &HashMap<&PathBuf, &str>,
) -> Vec<NodeKey> {
    let callee = &call.callee;
    let caller_lang = analysis.language;

    // Strip scope prefix for qualified calls like Self::method(), Type::new(),
    // HashMap::new(), module::func(). The name index is keyed on bare names
    // (from Symbol.name), but call captures include the full scoped_identifier.
    let bare_name = callee.rsplit("::").next().unwrap_or(callee);

    if let Some(keys) = name_index.get(bare_name) {
        // Prefer same-file matches
        let same_file: Vec<NodeKey> = keys
            .iter()
            .filter(|(path, _, _)| *path == analysis.path)
            .cloned()
            .collect();
        if !same_file.is_empty() {
            return same_file;
        }
        // Cross-file matches filtered to same language only
        keys.iter()
            .filter(|(path, _, _)| lang_index.get(path).copied() == Some(caller_lang))
            .cloned()
            .collect()
    } else {
        vec![]
    }
}
