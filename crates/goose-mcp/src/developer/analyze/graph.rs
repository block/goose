use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::developer::analyze::types::{AnalysisResult, CallChain};

#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    callers: HashMap<String, Vec<(PathBuf, usize, String)>>,
    callees: HashMap<String, Vec<(PathBuf, usize, String)>>,
    pub definitions: HashMap<String, Vec<(PathBuf, usize)>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_from_results(results: &[(PathBuf, AnalysisResult)]) -> Self {
        tracing::debug!("Building call graph from {} files", results.len());
        let mut graph = Self::new();

        for (file_path, result) in results {
            // Record definitions
            for func in &result.functions {
                graph
                    .definitions
                    .entry(func.name.clone())
                    .or_default()
                    .push((file_path.clone(), func.line));
            }

            for class in &result.classes {
                graph
                    .definitions
                    .entry(class.name.clone())
                    .or_default()
                    .push((file_path.clone(), class.line));
            }

            // Record call relationships
            for call in &result.calls {
                let caller = call
                    .caller_name
                    .clone()
                    .unwrap_or_else(|| "<module>".to_string());

                // Add to callers map (who calls this function)
                graph
                    .callers
                    .entry(call.callee_name.clone())
                    .or_default()
                    .push((file_path.clone(), call.line, caller.clone()));

                // Add to callees map (what this function calls)
                if caller != "<module>" {
                    graph.callees.entry(caller).or_default().push((
                        file_path.clone(),
                        call.line,
                        call.callee_name.clone(),
                    ));
                }
            }
        }

        tracing::trace!(
            "Graph built: {} definitions, {} caller entries, {} callee entries",
            graph.definitions.len(),
            graph.callers.len(),
            graph.callees.len()
        );

        graph
    }

    pub fn find_incoming_chains(&self, symbol: &str, max_depth: u32) -> Vec<CallChain> {
        tracing::trace!(
            "Finding incoming chains for {} with depth {}",
            symbol,
            max_depth
        );

        if max_depth == 0 {
            return vec![];
        }

        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct callers
        if let Some(direct_callers) = self.callers.get(symbol) {
            for (file, line, caller) in direct_callers {
                let initial_path = vec![(file.clone(), *line, caller.clone(), symbol.to_string())];

                if max_depth == 1 {
                    chains.push(CallChain { path: initial_path });
                } else {
                    queue.push_back((caller.clone(), initial_path, 1));
                }
            }
        }

        // BFS to find deeper chains
        while let Some((current_symbol, path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                chains.push(CallChain { path });
                continue;
            }

            // Avoid cycles
            if visited.contains(&current_symbol) {
                chains.push(CallChain { path }); // Still record the path we found
                continue;
            }
            visited.insert(current_symbol.clone());

            // Find who calls the current symbol
            if let Some(callers) = self.callers.get(&current_symbol) {
                for (file, line, caller) in callers {
                    let mut new_path =
                        vec![(file.clone(), *line, caller.clone(), current_symbol.clone())];
                    new_path.extend(path.clone());

                    if depth + 1 >= max_depth {
                        chains.push(CallChain { path: new_path });
                    } else {
                        queue.push_back((caller.clone(), new_path, depth + 1));
                    }
                }
            } else {
                // No more callers, this is a chain end
                chains.push(CallChain { path });
            }
        }

        tracing::trace!("Found {} incoming chains", chains.len());
        chains
    }

    pub fn find_outgoing_chains(&self, symbol: &str, max_depth: u32) -> Vec<CallChain> {
        tracing::trace!(
            "Finding outgoing chains for {} with depth {}",
            symbol,
            max_depth
        );

        if max_depth == 0 {
            return vec![];
        }

        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with what this symbol calls
        if let Some(direct_callees) = self.callees.get(symbol) {
            for (file, line, callee) in direct_callees {
                let initial_path = vec![(file.clone(), *line, symbol.to_string(), callee.clone())];

                if max_depth == 1 {
                    chains.push(CallChain { path: initial_path });
                } else {
                    queue.push_back((callee.clone(), initial_path, 1));
                }
            }
        }

        // BFS to find deeper chains
        while let Some((current_symbol, path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                chains.push(CallChain { path });
                continue;
            }

            // Avoid cycles
            if visited.contains(&current_symbol) {
                chains.push(CallChain { path });
                continue;
            }
            visited.insert(current_symbol.clone());

            // Find what the current symbol calls
            if let Some(callees) = self.callees.get(&current_symbol) {
                for (file, line, callee) in callees {
                    let mut new_path = path.clone();
                    new_path.push((file.clone(), *line, current_symbol.clone(), callee.clone()));

                    if depth + 1 >= max_depth {
                        chains.push(CallChain { path: new_path });
                    } else {
                        queue.push_back((callee.clone(), new_path, depth + 1));
                    }
                }
            } else {
                // No more callees, this is a chain end
                chains.push(CallChain { path });
            }
        }

        tracing::trace!("Found {} outgoing chains", chains.len());
        chains
    }
}
