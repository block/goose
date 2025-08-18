#![cfg(feature = "repo-index")]

use goose::repo_index::service::RepoIndexService; use goose::repo_index::{RepoIndexOptions, RepoIndexOutput};
use std::path::Path;

// Consolidated tests operating over the shared example repository fixtures.

fn build_example_service() -> RepoIndexService {
    // examples/example-treesitter-repo/src relative to crate
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/example-treesitter-repo/src");
    let root = root.canonicalize().expect("canonical example path");
    let mut sink = std::io::sink();
    let opts = goose::repo_index::RepoIndexOptions { root: root.as_path(), include_langs: None, output: goose::repo_index::RepoIndexOutput::Writer(&mut sink), progress: None };
    let (svc, _stats) = RepoIndexService::build(opts).expect("build index");
    svc
}

#[test]
fn test_languages_present() {
    let svc = build_example_service();
    let expected = ["rust","python","javascript","typescript","swift","java","cpp","c_sharp","go"];
    for l in expected.iter() {
        assert!(svc.files.iter().any(|f| f.language==*l), "language missing: {l}");
    }
}

#[test]
fn test_python_unresolved_imports() {
    let svc = build_example_service();
    let idx = svc.files.iter().position(|f| f.path.ends_with("python/test.py")).expect("python test file");
    let unresolved = svc.unresolved_imports_for_file_index(idx);
    assert!(unresolved.iter().any(|u| u=="math"), "expected math unresolved (got {:?})", unresolved);
}

#[test]
fn test_basic_symbol_search() {
    let svc = build_example_service();
    let hits = svc.search_symbol_exact("greet");
    assert!(!hits.is_empty(), "expected greet function");
}

#[test]
fn test_file_entities_exist() {
    let svc = build_example_service();
    // ensure each file has a pseudo-entity entry
    for f in &svc.files {
        let file_entity = svc.file_entities[f.id as usize];
        let ent = &svc.entities[file_entity as usize];
        assert_eq!(ent.kind.as_str(), "file");
    }
}

#[test]
fn test_rust_call_graph_traversal_present() {
    let svc = build_example_service();
    // Just ensure traversal functions run without panic; pick first non-file entity
    if let Some(ent) = svc.entities.iter().find(|e| e.kind.as_str()!="file") {
        let _callees = svc.callees_up_to(ent.id, 1);
        let _callers = svc.callers_up_to(ent.id, 1);
    }
}

#[test]
fn test_import_edges_any_present() {
    let svc = build_example_service();
    // At least one file should have import edges (python or others)
    let any = svc.entities.iter().filter(|e| e.kind.as_str()=="file").any(|e| !svc.imported_files(e.id).is_empty());
    assert!(any, "expected at least one file with import edges");
}

use std::fs::File as FsFile; use std::io::Write; use tempfile::tempdir;

// Helper for ad-hoc temp repos
fn build_temp_repo(root: &std::path::Path) -> RepoIndexService {
    let mut sink = std::io::sink();
    let opts = RepoIndexOptions { root, include_langs: None, output: RepoIndexOutput::Writer(&mut sink), progress: None };
    let (svc, _stats) = RepoIndexService::build(opts).expect("build temp repo");
    svc
}

#[test]
fn test_cross_file_unique_call_and_traversal() {
    let dir = tempdir().unwrap();
    // file a defines unique function alpha & beta
    let file_a = dir.path().join("a.rs");
    FsFile::create(&file_a).unwrap().write_all(b"pub fn alpha() {}\npub fn beta() {}\n").unwrap();
    // file b calls alpha
    let file_b = dir.path().join("b.rs");
    FsFile::create(&file_b).unwrap().write_all(b"fn gamma() { alpha(); }\n").unwrap();
    let svc = build_temp_repo(dir.path());
    let alpha = svc.search_symbol_exact("alpha");
    let gamma = svc.search_symbol_exact("gamma");
    assert_eq!(alpha.len(), 1, "alpha should be unique");
    assert_eq!(gamma.len(), 1, "gamma should be indexed once");
    let alpha_id = alpha[0].id; let gamma_id = gamma[0].id;
    assert_ne!(alpha[0].file_id, gamma[0].file_id, "Ensure cross-file scenario");
    assert!(svc.outgoing_calls(gamma_id).contains(&alpha_id), "gamma should call alpha");
    assert!(svc.incoming_calls(alpha_id).contains(&gamma_id), "alpha should have reverse edge");
    assert!(svc.callees_up_to(gamma_id,1).contains(&alpha_id));
    assert!(svc.callers_up_to(alpha_id,1).contains(&gamma_id));
}

#[test]
fn test_python_import_edges_and_unresolved() {
    let dir = tempdir().unwrap();
    FsFile::create(dir.path().join("a.py")).unwrap().write_all(b"import b, json as jsn\nfrom . import b as bmod\nfrom a import something\n").unwrap();
    FsFile::create(dir.path().join("b.py")).unwrap().write_all(b"def foo():\n    return 1\n").unwrap();
    let svc = build_temp_repo(dir.path());
    // locate file entities by filename
    let mut a_file_entity=None; let mut b_file_entity=None;
    for e in &svc.entities { if e.kind.as_str()=="file" { let stem=std::path::Path::new(&svc.files[e.file_id as usize].path).file_stem().unwrap().to_str().unwrap(); if stem=="a" { a_file_entity=Some(e.id);} if stem=="b" { b_file_entity=Some(e.id);} }}
    let a_id = a_file_entity.expect("a file entity");
    let b_id = b_file_entity.expect("b file entity");
    assert!(svc.imported_files(a_id).contains(&b_id), "a should import b");
    let unresolved = svc.unresolved_imports_for_file_index(0);
    assert!(unresolved.iter().any(|u| u=="json"), "json expected unresolved, got {:?}", unresolved);
}

#[test]
fn test_rust_mod_and_use_resolution() {
    let dir = tempdir().unwrap();
    FsFile::create(dir.path().join("lib.rs")).unwrap().write_all(b"mod foo;\nuse crate::foo;\n").unwrap();
    FsFile::create(dir.path().join("foo.rs")).unwrap().write_all(b"pub fn bar(){}\n").unwrap();
    let svc = build_temp_repo(dir.path());
    let mut lib_entity=None; let mut foo_entity=None;
    for e in &svc.entities { if e.kind.as_str()=="file" { let stem=std::path::Path::new(&svc.files[e.file_id as usize].path).file_stem().unwrap().to_str().unwrap(); if stem=="lib" { lib_entity=Some(e.id);} if stem=="foo" { foo_entity=Some(e.id);} }}
    let lib_id=lib_entity.unwrap(); let foo_id=foo_entity.unwrap();
    assert!(svc.imported_files(lib_id).contains(&foo_id), "lib should import foo module");
}

#[test]
fn test_cpp_local_include() {
    let dir = tempdir().unwrap();
    FsFile::create(dir.path().join("add.h")).unwrap().write_all(b"#pragma once\nint add(int a,int b);").unwrap();
    FsFile::create(dir.path().join("main.cpp")).unwrap().write_all(b"#include \"add.h\"\nint add(int a,int b){return a+b;}").unwrap();
    let svc = build_temp_repo(dir.path());
    let mut main_entity=None; let mut add_entity=None;
    for e in &svc.entities { if e.kind.as_str()=="file" { let stem=std::path::Path::new(&svc.files[e.file_id as usize].path).file_stem().unwrap().to_str().unwrap(); if stem=="main" { main_entity=Some(e.id);} if stem=="add" { add_entity=Some(e.id);} }}
    let main_id=main_entity.unwrap(); let add_id=add_entity.unwrap();
    assert!(svc.imported_files(main_id).contains(&add_id), "main should import add via include");
}

#[test]
fn test_java_csharp_swift_basic() {
    let dir = tempdir().unwrap();
    FsFile::create(dir.path().join("Util.java")).unwrap().write_all(b"package p; public class Util {} ").unwrap();
    FsFile::create(dir.path().join("Util.cs")).unwrap().write_all(b"public class Util {} ").unwrap();
    FsFile::create(dir.path().join("Program.cs")).unwrap().write_all(b"using System;\nusing Util;\nclass Program {}\n").unwrap();
    FsFile::create(dir.path().join("main.swift")).unwrap().write_all(b"import Util\n@testable import XCTest").unwrap();
    let svc = build_temp_repo(dir.path());
    // Identify file entity ids
    let mut program_entity=None; let mut util_java=None; let mut swift_main=None;
    for e in &svc.entities { if e.kind.as_str()=="file" { let stem=std::path::Path::new(&svc.files[e.file_id as usize].path).file_stem().unwrap().to_str().unwrap(); match stem { "Program" => program_entity=Some(e.id), "Util" => { if util_java.is_none() { util_java=Some(e.id);} }, "main" => swift_main=Some(e.id), _=>{} } }}
    let program_id=program_entity.unwrap(); let swift_id=swift_main.unwrap();
    // Swift should import Util (heuristic expected) BUT allow fallback if not resolved (then should appear unresolved)
    let swift_idx = svc.files.iter().position(|f| f.path.ends_with("main.swift")).unwrap();
    let swift_unresolved = svc.unresolved_imports_for_file_index(swift_idx);
    let swift_imports = svc.imported_files(swift_id);
    assert!(!swift_imports.is_empty() || !swift_unresolved.is_empty(), "expected swift main to have some import signal");
    // C# heuristic may not resolve local Util; ensure we at least parsed some using statements (either edges or unresolved)
    let csharp_unresolved = svc.unresolved_imports_for_file_index(svc.files.iter().position(|f| f.path.ends_with("Program.cs")).unwrap());
    let program_imports = svc.imported_files(program_id);
    assert!(!program_imports.is_empty() || !csharp_unresolved.is_empty(), "expected either resolved or unresolved imports for Program.cs");
}

#[test]
fn test_pagerank_variance() {
    let svc = build_example_service();
    let mut ranks: Vec<f32> = svc.entities.iter().filter(|e| e.kind.as_str()!="file").map(|e| e.rank).collect();
    assert!(!ranks.is_empty(), "expected non-file entities to compute ranks");
    ranks.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let min = ranks.first().copied().unwrap();
    let max = ranks.last().copied().unwrap();
    let delta = max - min;
    assert!(delta > 1e-6, "expected rank variance > 1e-6 (min={min}, max={max})");
}

#[test]
fn test_env_override_rank_weights() {
    use std::env;
    // Save originals
    let orig = env::var("GOOSE_REPO_RANK_CALL_WEIGHT").ok();
    env::set_var("GOOSE_REPO_RANK_CALL_WEIGHT", "2.0");
    env::set_var("GOOSE_REPO_RANK_IMPORT_WEIGHT", "0.1");
    env::set_var("GOOSE_REPO_RANK_CONTAINMENT_WEIGHT", "0.05");
    env::set_var("GOOSE_REPO_RANK_DAMPING", "0.9");
    env::set_var("GOOSE_REPO_RANK_ITERATIONS", "5");
    // Build small temp repo to minimize interference
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), b"pub fn a(){}\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), b"fn b(){ a(); }\n").unwrap();
    let mut sink = std::io::sink();
    let opts = goose::repo_index::RepoIndexOptions { root: dir.path(), include_langs: None, output: goose::repo_index::RepoIndexOutput::Writer(&mut sink), progress: None };
    let (svc, _stats) = RepoIndexService::build(opts).expect("build with env overrides");
    assert!((svc.rank_weights.call - 2.0).abs() < 1e-6, "call weight override not applied");
    assert!((svc.rank_weights.import - 0.1).abs() < 1e-6, "import weight override not applied");
    assert!((svc.rank_weights.containment - 0.05).abs() < 1e-6, "containment weight override not applied");
    assert!((svc.rank_weights.damping - 0.9).abs() < 1e-6, "damping override not applied");
    assert_eq!(svc.rank_weights.iterations, 5, "iterations override not applied");
    // Restore / clear
    if let Some(val) = orig { env::set_var("GOOSE_REPO_RANK_CALL_WEIGHT", val); } else { env::remove_var("GOOSE_REPO_RANK_CALL_WEIGHT"); }
    env::remove_var("GOOSE_REPO_RANK_IMPORT_WEIGHT");
    env::remove_var("GOOSE_REPO_RANK_CONTAINMENT_WEIGHT");
    env::remove_var("GOOSE_REPO_RANK_DAMPING");
    env::remove_var("GOOSE_REPO_RANK_ITERATIONS");
}
