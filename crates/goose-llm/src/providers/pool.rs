use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::Value;
use tokio::sync::Semaphore;
use tracing::debug;

use super::{
    base::{Provider, ProviderCompleteResponse, ProviderExtractResponse},
    errors::ProviderError,
    factory,
};
use crate::{message::Message, model::ModelConfig, types::core::Tool};

/// Statistics for the provider pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub created: usize,
    pub borrowed: usize,
    pub returned: usize,
    pub errors: usize,
    pub max_pool_size: usize,
    pub current_pool_size: usize,
    pub waiting: usize,
}

/// A pool entry containing a provider and metadata
#[derive(Debug)]
struct PooledProvider {
    provider: Arc<dyn Provider>,
    created_at: Instant,
    last_used: Instant,
    use_count: usize,
}

impl PooledProvider {
    fn new(provider: Arc<dyn Provider>) -> Self {
        let now = Instant::now();
        Self {
            provider,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }

    fn used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }
}

/// Configuration for the provider pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub max_idle_time: Duration,
    pub max_lifetime: Duration,
    pub max_uses: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            max_idle_time: Duration::from_secs(300),   // 5 minutes
            max_lifetime: Duration::from_secs(3600),   // 1 hour
            max_uses: 100,
        }
    }
}

/// ProviderPool manages a pool of provider instances
#[derive(Debug)]
pub struct ProviderPool {
    // Pool configuration
    config: PoolConfig,
    
    // Provider creation parameters
    provider_name: String,
    provider_config: Value,
    model_config: ModelConfig,
    
    // Pool state
    available: RwLock<Vec<PooledProvider>>,
    in_use: AtomicUsize,
    stats: Arc<RwLock<PoolStats>>,
    
    // Concurrency control
    semaphore: Arc<Semaphore>,
}

impl ProviderPool {
    /// Create a new provider pool
    pub fn new(
        provider_name: String,
        provider_config: Value,
        model_config: ModelConfig,
        config: PoolConfig,
    ) -> Self {
        let stats = Arc::new(RwLock::new(PoolStats {
            max_pool_size: config.max_size,
            ..PoolStats::default()
        }));

        Self {
            config: config.clone(),
            provider_name,
            provider_config,
            model_config,
            available: RwLock::new(Vec::with_capacity(config.max_size)),
            in_use: AtomicUsize::new(0),
            stats,
            semaphore: Arc::new(Semaphore::new(config.max_size)),
        }
    }

    /// Get a provider from the pool
    pub async fn get(self: Arc<Self>) -> Result<PooledProviderGuard, ProviderError> {
        // Acquire a permit from the semaphore to limit concurrent requests
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to acquire semaphore: {}", e)))?;

        // Update waiting count
        {
            let mut stats = self.stats.write().unwrap();
            stats.waiting += 1;
        }

        // Try to get a provider from the pool
        let provider = self.get_or_create_provider().await?;

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.waiting -= 1;
            stats.borrowed += 1;
            stats.current_pool_size = self.available.read().unwrap().len() + self.in_use.load(Ordering::SeqCst);
        }

        // Return the provider wrapped in a guard
        Ok(PooledProviderGuard {
            pool: self,
            provider: Some(provider),
            returned: false,
        })
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.stats.read().unwrap().clone()
    }

    /// Internal method to get a provider from the pool or create a new one
    async fn get_or_create_provider(&self) -> Result<Arc<dyn Provider>, ProviderError> {
        // Try to get a provider from the pool first
        if let Some(mut pooled) = self.take_available() {
            // Check if the provider is still valid
            let now = Instant::now();
            let idle_time = now.duration_since(pooled.last_used);
            let lifetime = now.duration_since(pooled.created_at);

            if idle_time > self.config.max_idle_time 
               || lifetime > self.config.max_lifetime 
               || pooled.use_count >= self.config.max_uses {
                // Provider expired, create a new one
                debug!(
                    "Provider expired: idle_time={:?}, lifetime={:?}, use_count={}",
                    idle_time, lifetime, pooled.use_count
                );
                drop(pooled); // Explicitly drop the expired provider
                self.create_new_provider().await
            } else {
                // Update usage stats
                pooled.used();
                self.in_use.fetch_add(1, Ordering::SeqCst);
                Ok(pooled.provider)
            }
        } else {
            // No available provider, create a new one
            self.create_new_provider().await
        }
    }

    /// Take an available provider from the pool
    fn take_available(&self) -> Option<PooledProvider> {
        let mut pool = self.available.write().unwrap();
        if !pool.is_empty() {
            Some(pool.remove(0))
        } else {
            None
        }
    }

    /// Create a new provider
    async fn create_new_provider(&self) -> Result<Arc<dyn Provider>, ProviderError> {
        let provider = factory::create(
            &self.provider_name,
            self.provider_config.clone(),
            self.model_config.clone(),
        )
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to create provider: {}", e)))?;

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.created += 1;
        }

        self.in_use.fetch_add(1, Ordering::SeqCst);
        Ok(provider)
    }

    /// Return a provider to the pool
    fn return_provider(&self, provider: Arc<dyn Provider>) {
        let mut pool = self.available.write().unwrap();
        if pool.len() < self.config.max_size {
            pool.push(PooledProvider::new(provider));
        }
        // else drop the provider (it goes over capacity)

        self.in_use.fetch_sub(1, Ordering::SeqCst);

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.returned += 1;
            stats.current_pool_size = pool.len() + self.in_use.load(Ordering::SeqCst);
        }
    }

    /// Handle an error with a provider
    fn handle_error(&self) {
        self.in_use.fetch_sub(1, Ordering::SeqCst);

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.errors += 1;
            stats.current_pool_size = self.available.read().unwrap().len() + self.in_use.load(Ordering::SeqCst);
        }
    }

    /// Clean up idle providers in the pool
    pub fn cleanup_idle(&self) -> usize {
        let mut pool = self.available.write().unwrap();
        let now = Instant::now();
        let initial_size = pool.len();
        
        // Remove providers that have been idle too long
        pool.retain(|p| {
            let idle_time = now.duration_since(p.last_used);
            let lifetime = now.duration_since(p.created_at);
            
            idle_time <= self.config.max_idle_time 
                && lifetime <= self.config.max_lifetime 
                && p.use_count < self.config.max_uses
        });
        
        let removed = initial_size - pool.len();
        
        // Update stats
        if removed > 0 {
            let mut stats = self.stats.write().unwrap();
            stats.current_pool_size = pool.len() + self.in_use.load(Ordering::SeqCst);
            debug!("Cleaned up {} idle providers", removed);
        }
        
        removed
    }
}

/// A guard that returns a provider to the pool when dropped
#[derive(Debug)]
pub struct PooledProviderGuard {
    pool: Arc<ProviderPool>,
    provider: Option<Arc<dyn Provider>>,
    returned: bool,
}

impl Drop for PooledProviderGuard {
    fn drop(&mut self) {
        // If the guard hasn't already returned the provider, return it now
        if !self.returned {
            if let Some(provider) = self.provider.take() {
                self.pool.return_provider(provider);
            }
        }
    }
}

impl PooledProviderGuard {
    /// Return the provider to the pool early
    pub fn return_to_pool(mut self) {
        if let Some(provider) = self.provider.take() {
            self.pool.return_provider(provider);
            self.returned = true;
        }
    }

    /// Mark the provider as having an error
    pub fn handle_error(mut self) {
        self.provider.take();
        self.pool.handle_error();
        self.returned = true;
    }
}

#[async_trait]
impl Provider for PooledProviderGuard {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<ProviderCompleteResponse, ProviderError> {
        match &self.provider {
            Some(provider) => provider.complete(system, messages, tools).await,
            None => Err(ProviderError::ExecutionError("Provider is not available".into())),
        }
    }

    async fn extract(
        &self, 
        system: &str, 
        messages: &[Message], 
        schema: &Value
    ) -> Result<ProviderExtractResponse, ProviderError> {
        match &self.provider {
            Some(provider) => provider.extract(system, messages, schema).await,
            None => Err(ProviderError::ExecutionError("Provider is not available".into())),
        }
    }
}

/// A provider pool manager that manages multiple pools
#[derive(Default)]
pub struct ProviderPoolManager {
    pools: Arc<RwLock<HashMap<String, Arc<ProviderPool>>>>,
    pool_configs: Arc<RwLock<HashMap<String, PoolConfig>>>,
    default_config: PoolConfig,
}

impl ProviderPoolManager {
    /// Create a new provider pool manager with default configuration
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            pool_configs: Arc::new(RwLock::new(HashMap::new())),
            default_config: PoolConfig::default(),
        }
    }

    /// Set the default pool configuration
    pub fn with_default_config(mut self, config: PoolConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Set a pool configuration for a specific provider
    pub fn set_pool_config(&self, provider_name: &str, config: PoolConfig) {
        let mut configs = self.pool_configs.write().unwrap();
        configs.insert(provider_name.to_string(), config);
    }

    /// Get the pool configuration for a provider
    fn get_pool_config(&self, provider_name: &str) -> PoolConfig {
        let configs = self.pool_configs.read().unwrap();
        configs
            .get(provider_name)
            .cloned()
            .unwrap_or_else(|| self.default_config.clone())
    }

    /// Get or create a provider pool
    pub fn get_or_create_pool(
        &self,
        provider_name: &str,
        provider_config: Value,
        model_config: ModelConfig,
    ) -> Arc<ProviderPool> {
        let mut pools = self.pools.write().unwrap();
        
        let key = self.create_pool_key(provider_name, &provider_config, &model_config);
        
        pools.entry(key.clone()).or_insert_with(|| {
            let config = self.get_pool_config(provider_name);
            Arc::new(ProviderPool::new(
                provider_name.to_string(),
                provider_config,
                model_config,
                config,
            ))
        }).clone()
    }

    /// Create a key for the pool based on provider name, config, and model
    fn create_pool_key(&self, provider_name: &str, provider_config: &Value, model_config: &ModelConfig) -> String {
        // Create a key that uniquely identifies this provider configuration
        // We use the provider name, a hash of the provider config, and the model name
        let config_hash = format!("{:x}", md5::compute(provider_config.to_string()));
        format!("{}:{}:{}", provider_name, config_hash, model_config.model_name)
    }

    /// Get statistics for all pools
    pub fn get_all_stats(&self) -> HashMap<String, PoolStats> {
        let pools = self.pools.read().unwrap();
        let mut stats = HashMap::with_capacity(pools.len());
        
        for (key, pool) in pools.iter() {
            stats.insert(key.clone(), pool.stats());
        }
        
        stats
    }

    /// Clean up idle providers in all pools
    pub fn cleanup_all_idle(&self) -> usize {
        let pools = self.pools.read().unwrap();
        let mut total_removed = 0;
        
        for pool in pools.values() {
            total_removed += pool.cleanup_idle();
        }
        
        total_removed
    }

    /// Start a background task that periodically cleans up idle providers
    pub fn start_cleanup_task(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let pools = self.pools.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                let pools_ref = pools.read().unwrap();
                for pool in pools_ref.values() {
                    let removed = pool.cleanup_idle();
                    if removed > 0 {
                        debug!("Cleaned up {} idle providers", removed);
                    }
                }
            }
        })
    }

}

// Create a global provider pool manager
lazy_static::lazy_static! {
    static ref GLOBAL_POOL_MANAGER: Mutex<Option<ProviderPoolManager>> = Mutex::new(None);
}

/// Initialize the global provider pool manager
pub fn init_global_pool_manager(config: Option<PoolConfig>) -> &'static ProviderPoolManager {
    let mut global = GLOBAL_POOL_MANAGER.lock();
    
    if global.is_none() {
        let manager = match config {
            Some(config) => ProviderPoolManager::new().with_default_config(config),
            None => ProviderPoolManager::new(),
        };
        
        // Start the cleanup task
        let cleanup_interval = Duration::from_secs(60); // 1 minute
        manager.start_cleanup_task(cleanup_interval);
        
        *global = Some(manager);
    }
    
    // SAFETY: This is safe because:
    // 1. We never remove the pool manager once initialized (it lives for the program duration)
    // 2. The mutex ensures thread-safe access to the manager
    // 3. The static reference is only to the contained manager which has a static lifetime
    let static_manager = unsafe { 
        let manager_ref = global.as_ref().unwrap();
        std::mem::transmute::<&ProviderPoolManager, &'static ProviderPoolManager>(manager_ref)
    };
    
    static_manager
}

/// Get the global provider pool manager, initializing it if needed
pub fn global_pool_manager() -> &'static ProviderPoolManager {
    let global = GLOBAL_POOL_MANAGER.lock();
    
    if let Some(manager) = &*global {
        // SAFETY: This is safe because the ProviderPoolManager is stored in a static Mutex
        // and lives for the entire program duration
        unsafe { std::mem::transmute::<&ProviderPoolManager, &'static ProviderPoolManager>(manager) }
    } else {
        drop(global); // Release lock before calling init
        init_global_pool_manager(None)
    }
}

