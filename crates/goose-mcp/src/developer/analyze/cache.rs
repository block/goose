use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::developer::analyze::types::AnalysisResult;

/// Cache for analysis results
#[derive(Clone)]
pub struct AnalysisCache {
    cache: Arc<Mutex<LruCache<CacheKey, AnalysisResult>>>,
    #[allow(dead_code)]
    max_size: usize,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct CacheKey {
    path: PathBuf,
    modified: SystemTime,
}

impl AnalysisCache {
    /// Create a new analysis cache with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        tracing::info!("Initializing analysis cache with size {}", max_size);

        let size = NonZeroUsize::new(max_size).unwrap_or_else(|| {
            tracing::warn!("Invalid cache size {}, using default 100", max_size);
            NonZeroUsize::new(100).unwrap()
        });

        Self {
            cache: Arc::new(Mutex::new(LruCache::new(size))),
            max_size,
        }
    }

    /// Get a cached result if it exists and is still valid
    pub fn get(&self, path: &PathBuf, modified: SystemTime) -> Option<AnalysisResult> {
        let mut cache = self.cache.lock().unwrap();
        let key = CacheKey {
            path: path.clone(),
            modified,
        };

        if let Some(result) = cache.get(&key) {
            tracing::trace!("Cache hit for {:?}", path);
            Some(result.clone())
        } else {
            tracing::trace!("Cache miss for {:?}", path);
            None
        }
    }

    /// Store a result in the cache
    pub fn put(&self, path: PathBuf, modified: SystemTime, result: AnalysisResult) {
        let mut cache = self.cache.lock().unwrap();
        let key = CacheKey {
            path: path.clone(),
            modified,
        };

        tracing::trace!("Caching result for {:?}", path);
        cache.put(key, result);
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        tracing::debug!("Cache cleared");
    }

    /// Get the current size of the cache
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.is_empty()
    }
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::developer::analyze::types::{ClassInfo, FunctionInfo};

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
}
