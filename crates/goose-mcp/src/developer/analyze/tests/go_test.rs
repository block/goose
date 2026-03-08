use crate::developer::analyze::graph::CallGraph;
use crate::developer::analyze::parser::{ElementExtractor, ParserManager};
use crate::developer::analyze::types::{AnalysisResult, ReferenceType};
use std::collections::HashSet;
use std::path::PathBuf;

fn parse_and_extract(code: &str) -> AnalysisResult {
    let manager = ParserManager::new();
    let tree = manager.parse(code, "go").unwrap();
    ElementExtractor::extract_with_depth(&tree, code, "go", "semantic", None).unwrap()
}

fn build_test_graph(files: Vec<(&str, &str)>) -> CallGraph {
    let manager = ParserManager::new();
    let results: Vec<_> = files
        .iter()
        .map(|(path, code)| {
            let tree = manager.parse(code, "go").unwrap();
            let result =
                ElementExtractor::extract_with_depth(&tree, code, "go", "semantic", None).unwrap();
            (PathBuf::from(*path), result)
        })
        .collect();
    CallGraph::build_from_results(&results)
}

#[test]
fn test_go_struct_and_method_tracking() {
    let code = r#"
package main

import "myapp/pkg/service"

type Config struct {
    Host string
    Port int
}

type Handler struct {
    Cfg *Config
    Svc *service.Widget
}

func (h *Handler) Start() error {
    return nil
}

func (h *Handler) Stop() error {
    return nil
}

func main() {
    cfg := Config{Host: "localhost", Port: 8080}
    handler := Handler{Cfg: &cfg}
    _ = handler.Start()
}
"#;

    let result = parse_and_extract(code);
    let graph = build_test_graph(vec![("test.go", code)]);

    assert_eq!(result.class_count, 2);
    let struct_names: HashSet<_> = result.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(struct_names.contains("Config"));
    assert!(struct_names.contains("Handler"));

    assert_eq!(result.function_count, 3);
    let method_names: HashSet<_> = result.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(method_names.contains("Start"));
    assert!(method_names.contains("Stop"));
    assert!(method_names.contains("main"));

    let handler_methods: Vec<_> = result
        .references
        .iter()
        .filter(|r| {
            r.ref_type == ReferenceType::MethodDefinition
                && r.associated_type.as_deref() == Some("Handler")
        })
        .collect();
    assert!(
        handler_methods.len() >= 2,
        "Expected at least 2 methods on Handler, found {}",
        handler_methods.len()
    );

    let field_type_refs: Vec<_> = result
        .references
        .iter()
        .filter(|r| r.ref_type == ReferenceType::FieldType)
        .collect();
    assert!(
        !field_type_refs.is_empty(),
        "Expected to find field type references"
    );

    let config_literals: Vec<_> = result
        .references
        .iter()
        .filter(|r| r.symbol == "Config" && r.ref_type == ReferenceType::TypeInstantiation)
        .collect();
    assert!(
        !config_literals.is_empty(),
        "Expected to find Config struct literals"
    );

    let incoming = graph.find_incoming_chains("Handler", 1);
    assert!(
        !incoming.is_empty(),
        "Expected to find incoming references to Handler"
    );

    let outgoing = graph.find_outgoing_chains("Handler", 1);
    assert!(!outgoing.is_empty(), "Expected to find methods on Handler");
}
