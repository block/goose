use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::developer::analyze::types::{AnalysisResult, CallChain};

// Minimal graph structure for focus mode only
#[derive(Debug, Clone)]
pub struct CallGraph {
    // Map from symbol name to its callers: Vec<(file, line, caller_function)>
    callers: HashMap<String, Vec<(PathBuf, usize, String)>>,
    // Map from symbol name to what it calls: Vec<(file, line, callee_function)>
    callees: HashMap<String, Vec<(PathBuf, usize, String)>>,
    // Map from symbol to its definition locations
    pub definitions: HashMap<String, Vec<(PathBuf, usize)>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            callers: HashMap::new(),
            callees: HashMap::new(),
            definitions: HashMap::new(),
        }
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
        tracing::trace!("Finding incoming chains for {} with depth {}", symbol, max_depth);
        
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
        tracing::trace!("Finding outgoing chains for {} with depth {}", symbol, max_depth);
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::developer::analyze::types::{CallInfo, ClassInfo, FunctionInfo};

    fn create_test_result(functions: Vec<&str>, calls: Vec<(&str, &str)>) -> AnalysisResult {
        AnalysisResult {
            functions: functions
                .into_iter()
                .map(|name| FunctionInfo {
                    name: name.to_string(),
                    line: 1,
                    params: vec![],
                })
                .collect(),
            classes: vec![],
            imports: vec![],
            calls: calls
                .into_iter()
                .map(|(caller, callee)| CallInfo {
                    caller_name: Some(caller.to_string()),
                    callee_name: callee.to_string(),
                    line: 1,
                    column: 0,
                    context: String::new(),
                })
                .collect(),
            references: vec![],
            function_count: 0,
            class_count: 0,
            line_count: 0,
            import_count: 0,
            main_line: None,
        }
    }

    #[test]
    fn test_simple_call_chain() {
        let results = vec![
            (
                PathBuf::from("test.rs"),
                create_test_result(vec!["a", "b", "c"], vec![("a", "b"), ("b", "c")]),
            ),
        ];

        let graph = CallGraph::build_from_results(&results);
        
        // Test incoming chains for 'c'
        let chains = graph.find_incoming_chains("c", 2);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].path.len(), 2); // b->c, a->b
        
        // Test outgoing chains for 'a'
        let chains = graph.find_outgoing_chains("a", 2);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].path.len(), 2); // a->b, b->c
    }

    #[test]
    fn test_circular_dependency() {
        let results = vec![
            (
                PathBuf::from("test.rs"),
                create_test_result(vec!["a", "b"], vec![("a", "b"), ("b", "a")]),
            ),
        ];

        let graph = CallGraph::build_from_results(&results);
        
        // Should handle cycles without infinite loop
        let chains = graph.find_incoming_chains("a", 3);
        assert!(!chains.is_empty());
    }
}
