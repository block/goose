use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::lock_or_recover;
use crate::developer::analyze::types::{AnalysisMode, AnalysisResult};

#[derive(Clone)]
pub struct AnalysisCache {
    cache: Arc<Mutex<LruCache<CacheKey, Arc<AnalysisResult>>>>,
    #[allow(dead_code)]
    max_size: usize,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct CacheKey {
    path: PathBuf,
    modified: SystemTime,
    mode: AnalysisMode,
}

impl AnalysisCache {
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

    pub fn get(
        &self,
        path: &PathBuf,
        modified: SystemTime,
        mode: &AnalysisMode,
    ) -> Option<AnalysisResult> {
        let mut cache = lock_or_recover(&self.cache, |c| c.clear());
        let key = CacheKey {
            path: path.clone(),
            modified,
            mode: *mode,
        };

        if let Some(result) = cache.get(&key) {
            tracing::trace!("Cache hit for {:?} in {:?} mode", path, mode);
            Some((**result).clone())
        } else {
            tracing::trace!("Cache miss for {:?} in {:?} mode", path, mode);
            None
        }
    }

    pub fn put(
        &self,
        path: PathBuf,
        modified: SystemTime,
        mode: &AnalysisMode,
        result: AnalysisResult,
    ) {
        let mut cache = lock_or_recover(&self.cache, |c| c.clear());
        let key = CacheKey {
            path: path.clone(),
            modified,
            mode: *mode,
        };

        tracing::trace!("Caching result for {:?} in {:?} mode", path, mode);
        cache.put(key, Arc::new(result));
    }

    pub fn clear(&self) {
        let mut cache = lock_or_recover(&self.cache, |c| c.clear());
        cache.clear();
        tracing::debug!("Cache cleared");
    }

    pub fn len(&self) -> usize {
        let cache = lock_or_recover(&self.cache, |c| c.clear());
        cache.len()
    }

    pub fn is_empty(&self) -> bool {
        let cache = lock_or_recover(&self.cache, |c| c.clear());
        cache.is_empty()
    }
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new(100)
    }
}
