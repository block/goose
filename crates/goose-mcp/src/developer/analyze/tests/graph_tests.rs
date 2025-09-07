// Tests for the graph module

use crate::developer::analyze::graph::CallGraph;
use crate::developer::analyze::tests::fixtures::create_test_result_with_calls;
use std::path::PathBuf;

#[test]
fn test_simple_call_chain() {
    let results = vec![(
        PathBuf::from("test.rs"),
        create_test_result_with_calls(vec!["a", "b", "c"], vec![("a", "b"), ("b", "c")]),
    )];

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
    let results = vec![(
        PathBuf::from("test.rs"),
        create_test_result_with_calls(vec!["a", "b"], vec![("a", "b"), ("b", "a")]),
    )];

    let graph = CallGraph::build_from_results(&results);

    // Should handle cycles without infinite loop
    let chains = graph.find_incoming_chains("a", 3);
    assert!(!chains.is_empty());
}

#[test]
fn test_empty_graph() {
    let graph = CallGraph::new();

    // Should return empty results for non-existent symbols
    let chains = graph.find_incoming_chains("nonexistent", 2);
    assert!(chains.is_empty());

    let chains = graph.find_outgoing_chains("nonexistent", 2);
    assert!(chains.is_empty());
}

#[test]
fn test_max_depth_zero() {
    let results = vec![(
        PathBuf::from("test.rs"),
        create_test_result_with_calls(vec!["a", "b"], vec![("a", "b")]),
    )];

    let graph = CallGraph::build_from_results(&results);

    // max_depth of 0 should return empty results
    let chains = graph.find_incoming_chains("b", 0);
    assert!(chains.is_empty());

    let chains = graph.find_outgoing_chains("a", 0);
    assert!(chains.is_empty());
}

#[test]
fn test_multiple_callers() {
    let results = vec![(
        PathBuf::from("test.rs"),
        create_test_result_with_calls(
            vec!["a", "b", "c", "target"],
            vec![("a", "target"), ("b", "target"), ("c", "target")],
        ),
    )];

    let graph = CallGraph::build_from_results(&results);

    // Should find all three callers
    let chains = graph.find_incoming_chains("target", 1);
    assert_eq!(chains.len(), 3);

    // Each chain should have exactly one call
    for chain in chains {
        assert_eq!(chain.path.len(), 1);
    }
}

#[test]
fn test_deep_chain() {
    let results = vec![(
        PathBuf::from("test.rs"),
        create_test_result_with_calls(
            vec!["a", "b", "c", "d", "e"],
            vec![("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")],
        ),
    )];

    let graph = CallGraph::build_from_results(&results);

    // Test various depths
    let chains = graph.find_incoming_chains("e", 1);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].path.len(), 1); // Just d->e

    let chains = graph.find_incoming_chains("e", 2);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].path.len(), 2); // c->d, d->e

    let chains = graph.find_incoming_chains("e", 4);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].path.len(), 4); // Full chain a->b->c->d->e
}
