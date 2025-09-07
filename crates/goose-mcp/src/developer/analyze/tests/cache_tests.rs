// Tests for the cache module

use crate::developer::analyze::cache::AnalysisCache;
use crate::developer::analyze::types::{AnalysisResult, FunctionInfo};
use std::path::PathBuf;
use std::time::SystemTime;

fn create_test_result() -> AnalysisResult {
    AnalysisResult {
        functions: vec![FunctionInfo {
            name: "test_func".to_string(),
            line: 1,
            params: vec![],
        }],
        classes: vec![],
        imports: vec![],
        calls: vec![],
        references: vec![],
        function_count: 1,
        class_count: 0,
        line_count: 10,
        import_count: 0,
        main_line: None,
    }
}

#[test]
fn test_cache_hit_miss() {
    let cache = AnalysisCache::new(10);
    let path = PathBuf::from("test.rs");
    let time = SystemTime::now();
    let result = create_test_result();

    // Initial miss
    assert!(cache.get(&path, time).is_none());

    // Store and hit
    cache.put(path.clone(), time, result.clone());
    assert!(cache.get(&path, time).is_some());

    // Different time = miss
    let later = time + std::time::Duration::from_secs(1);
    assert!(cache.get(&path, later).is_none());
}

#[test]
fn test_cache_eviction() {
    let cache = AnalysisCache::new(2);
    let result = create_test_result();
    let time = SystemTime::now();

    // Fill cache
    cache.put(PathBuf::from("file1.rs"), time, result.clone());
    cache.put(PathBuf::from("file2.rs"), time, result.clone());
    assert_eq!(cache.len(), 2);

    // Add third item, should evict first
    cache.put(PathBuf::from("file3.rs"), time, result.clone());
    assert_eq!(cache.len(), 2);

    // First item should be evicted
    assert!(cache.get(&PathBuf::from("file1.rs"), time).is_none());
    assert!(cache.get(&PathBuf::from("file2.rs"), time).is_some());
    assert!(cache.get(&PathBuf::from("file3.rs"), time).is_some());
}

#[test]
fn test_cache_clear() {
    let cache = AnalysisCache::new(10);
    let path = PathBuf::from("test.rs");
    let time = SystemTime::now();
    let result = create_test_result();

    cache.put(path.clone(), time, result);
    assert!(!cache.is_empty());

    cache.clear();
    assert!(cache.is_empty());
    assert!(cache.get(&path, time).is_none());
}

#[test]
fn test_cache_default() {
    let cache = AnalysisCache::default();
    assert!(cache.is_empty());

    // Default cache should work normally
    let path = PathBuf::from("test.rs");
    let time = SystemTime::now();
    let result = create_test_result();

    cache.put(path.clone(), time, result);
    assert!(cache.get(&path, time).is_some());
}
