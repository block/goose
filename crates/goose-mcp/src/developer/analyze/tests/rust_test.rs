use crate::developer::analyze::graph::CallGraph;
use crate::developer::analyze::parser::{ElementExtractor, ParserManager};
use crate::developer::analyze::types::{AnalysisResult, ReferenceType};
use std::collections::HashSet;
use std::path::PathBuf;

fn parse_and_extract(code: &str) -> AnalysisResult {
    let manager = ParserManager::new();
    let tree = manager.parse(code, "rust").unwrap();
    ElementExtractor::extract_with_depth(&tree, code, "rust", "semantic", None).unwrap()
}

fn build_test_graph(files: Vec<(&str, &str)>) -> CallGraph {
    let manager = ParserManager::new();
    let results: Vec<_> = files
        .iter()
        .map(|(path, code)| {
            let tree = manager.parse(code, "rust").unwrap();
            let result =
                ElementExtractor::extract_with_depth(&tree, code, "rust", "semantic", None)
                    .unwrap();
            (PathBuf::from(*path), result)
        })
        .collect();
    CallGraph::build_from_results(&results)
}

#[test]
fn test_rust_self_parameter_type_resolution() {
    // Test that self parameters correctly resolve to their impl type
    let code = r#"
struct MyStruct {
    value: i32,
}

impl MyStruct {
    fn method_with_self(&self) -> i32 {
        self.value
    }

    fn method_with_mut_self(&mut self) {
        self.value += 1;
    }

    fn associated_function() -> Self {
        MyStruct { value: 0 }
    }
}
"#;

    let result = parse_and_extract(code);

    // Find method references with self parameters
    let self_methods: Vec<_> = result
        .references
        .iter()
        .filter(|r| r.ref_type == ReferenceType::MethodDefinition)
        .collect();

    // Should find both methods with self parameters
    assert_eq!(
        self_methods.len(),
        2,
        "Expected 2 methods with self parameters"
    );

    // Both should be associated with MyStruct
    for method_ref in &self_methods {
        assert_eq!(
            method_ref.associated_type.as_deref(),
            Some("MyStruct"),
            "Method {} should be associated with MyStruct",
            method_ref.symbol
        );
    }

    // Verify the specific methods
    let method_names: HashSet<_> = self_methods.iter().map(|r| r.symbol.as_str()).collect();
    assert!(method_names.contains("method_with_self"));
    assert!(method_names.contains("method_with_mut_self"));
}

#[test]
fn test_rust_struct_and_impl_tracking() {
    let code = r#"
struct Config {
    host: String,
    port: u16,
}

struct Handler {
    cfg: Config,
}

impl Handler {
    fn new(cfg: Config) -> Self {
        Handler { cfg }
    }

    fn start(&self) -> Result<(), String> {
        Ok(())
    }
}

fn main() {
    let cfg = Config { host: "localhost".to_string(), port: 8080 };
    let handler = Handler::new(cfg);
    let _ = handler.start();
}
"#;

    let result = parse_and_extract(code);
    let graph = build_test_graph(vec![("test.rs", code)]);

    // Test struct extraction (includes impl blocks)
    assert_eq!(result.class_count, 3); // Config, Handler, impl Handler
    let struct_names: HashSet<_> = result.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(struct_names.contains("Config"));
    assert!(struct_names.contains("Handler"));

    // Test method extraction
    let method_names: HashSet<_> = result.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(method_names.contains("new"));
    assert!(method_names.contains("start"));
    assert!(method_names.contains("main"));

    // Test method-to-type associations (only methods with self parameter)
    let handler_methods: Vec<_> = result
        .references
        .iter()
        .filter(|r| {
            r.ref_type == ReferenceType::MethodDefinition
                && r.associated_type.as_deref() == Some("Handler")
        })
        .collect();
    assert!(
        !handler_methods.is_empty(),
        "Expected at least 1 method on Handler (start), found {}",
        handler_methods.len()
    );

    // Verify the method is 'start' (new doesn't have self, so it's not tracked)
    assert!(
        handler_methods.iter().any(|r| r.symbol == "start"),
        "Expected to find 'start' method on Handler"
    );

    // Test field type tracking
    let field_type_refs: Vec<_> = result
        .references
        .iter()
        .filter(|r| r.ref_type == ReferenceType::FieldType)
        .collect();
    assert!(
        !field_type_refs.is_empty(),
        "Expected to find field type references"
    );

    // Test struct instantiation
    let config_literals: Vec<_> = result
        .references
        .iter()
        .filter(|r| r.symbol == "Config" && r.ref_type == ReferenceType::TypeInstantiation)
        .collect();
    assert!(
        !config_literals.is_empty(),
        "Expected to find Config struct literals"
    );

    // Test call graph integration
    let incoming = graph.find_incoming_chains("Handler", 1);
    assert!(
        !incoming.is_empty(),
        "Expected to find incoming references to Handler"
    );

    let outgoing = graph.find_outgoing_chains("Handler", 1);
    assert!(!outgoing.is_empty(), "Expected to find methods on Handler");
}
