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
