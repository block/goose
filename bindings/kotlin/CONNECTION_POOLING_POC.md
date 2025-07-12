# Connection Pooling Proof of Concept for Goose LLM

This document outlines a proof-of-concept implementation for adding connection pooling to the goose-llm FFI bindings in Kotlin.

## Architecture Overview

The connection pooling system would consist of:

1. **Provider Pool in Rust**:
   - Thread-safe pool of provider instances
   - Configurable pool size, idle timeout, and connection limits
   - Auto-cleanup of idle connections
   
2. **Kotlin Bindings**:
   - Direct access to pool configuration functions
   - Connection pool statistics
   - Integration with completion requests

## Implementation

### Rust Side (crates/goose-llm)

1. Create `ProviderPool` in `providers/pool.rs`:
```rust
pub struct ProviderPool {
    providers: Arc<Mutex<HashMap<String, Vec<PoolEntry>>>>,
    config: PoolConfig,
}

pub struct PoolConfig {
    pub max_size: u32,
    pub max_idle_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub max_uses: u32,
}

impl ProviderPool {
    // Get or create a provider
    pub async fn get_provider(&self, name: &str, config: serde_json::Value, model: ModelConfig) -> Result<Arc<dyn Provider>, ProviderError>;
    
    // Return a provider to the pool
    pub fn return_provider(&self, provider: Arc<dyn Provider>);
    
    // Clean up idle providers
    pub fn cleanup_idle(&self);
    
    // Get pool statistics
    pub fn get_stats(&self) -> PoolStats;
}
```

2. Add global pool manager in `completion.rs`:
```rust
// Initialize the pool
#[uniffi::export]
pub fn init_provider_pool() {
    // Initialize the global provider pool
    let _ = PROVIDER_POOL.get_or_init(|| Arc::new(ProviderPool::new(PoolConfig::default())));
}

// Configure the pool
#[uniffi::export]
pub fn configure_provider_pool(
    max_size: u32, 
    max_idle_seconds: u64,
    max_lifetime_seconds: u64,
    max_uses: u32,
) {
    let config = PoolConfig {
        max_size,
        max_idle_seconds,
        max_lifetime_seconds,
        max_uses,
    };
    
    if let Some(pool) = PROVIDER_POOL.get() {
        pool.update_config(config);
    } else {
        let _ = PROVIDER_POOL.get_or_init(|| Arc::new(ProviderPool::new(config)));
    }
}

// Get statistics about the provider pool
#[uniffi::export]
pub fn get_pool_stats() -> String {
    if let Some(pool) = PROVIDER_POOL.get() {
        format!("{:?}", pool.get_stats())
    } else {
        "Provider pool not initialized".into()
    }
}
```

3. Update `CompletionRequest` to include a pool option:
```rust
pub struct CompletionRequest {
    // existing fields...
    pub use_pool: Option<bool>,
}
```

4. Update `completion` function to use pooled providers:
```rust
#[uniffi::export(async_runtime = "tokio")]
pub async fn completion(req: CompletionRequest) -> Result<CompletionResponse, CompletionError> {
    // Check if we should use pooling
    let use_pool = req.use_pool.unwrap_or(true);
    
    let provider = if use_pool && PROVIDER_POOL.get().is_some() {
        // Get provider from pool
        PROVIDER_POOL
            .get()
            .unwrap()
            .get_provider(&req.provider_name, req.provider_config.clone(), req.model_config.clone())
            .await
            .map_err(|e| CompletionError::Provider(e))?
    } else {
        // Create provider directly
        create_provider(&req.provider_name, req.provider_config.clone(), req.model_config.clone())
            .map_err(|_| CompletionError::UnknownProvider(req.provider_name.clone()))?
    };
    
    // Rest of completion function...
}
```

### Kotlin Side (bindings/kotlin)

1. Add provider pool functions in `uniffi_goose_llm.kt`:
```kotlin
/**
 * Initialize the provider pool with default configuration
 */
fun initProviderPool() {
    // FFI call to init_provider_pool
}

/**
 * Configure the provider pool with custom settings
 */
fun configureProviderPool(
    maxSize: Int,
    maxIdleSeconds: Long,
    maxLifetimeSeconds: Long,
    maxUses: Int
) {
    // FFI call to configure_provider_pool
}

/**
 * Get statistics about the provider pool
 */
fun getPoolStats(): String {
    // FFI call to get_pool_stats
}

/**
 * Create a completion request with optional pool setting
 */
fun createCompletionRequest(
    // existing parameters
    usePool: Boolean? = null
): CompletionRequest {
    // Create request with usePool parameter
}
```

2. Create helper class for managing pool:
```kotlin
class ProviderPool {
    companion object {
        /**
         * Initialize the provider pool with default settings
         */
        fun initialize() {
            initProviderPool()
        }

        /**
         * Configure the provider pool with custom settings
         */
        fun configure(maxSize: Int = 10, maxIdleSeconds: Long = 300, 
                      maxLifetimeSeconds: Long = 3600, maxUses: Int = 100) {
            configureProviderPool(maxSize, maxIdleSeconds, maxLifetimeSeconds, maxUses)
        }

        /**
         * Get statistics about the current connection pool
         */
        fun stats(): String {
            return getPoolStats()
        }
    }
}
```

## Usage Example

```kotlin
// Initialize and configure the pool
ProviderPool.initialize()
ProviderPool.configure(maxSize = 20, maxIdleSeconds = 300)

// Create a completion request with pooling
val request = createCompletionRequest(
    providerName = "openai",
    providerConfig = providerConfig,
    modelConfig = modelConfig,
    systemPreamble = "You are a helpful assistant.",
    messages = messages,
    extensions = emptyList(),
    usePool = true // Enable connection pooling
)

// Process the completion
val response = completion(request)

// Get pool statistics
println(ProviderPool.stats())
```

## Performance Benefits

The connection pooling implementation would provide the following benefits:

1. **Reduced Latency**: By reusing existing connections, we eliminate the overhead of creating new providers for each request.

2. **Higher Throughput**: More efficient connection handling means more requests can be processed in parallel.

3. **Less Resource Usage**: Fewer connections means less memory usage and fewer file descriptors.

4. **Improved Stability**: Better handling of connection limits helps avoid rate limiting issues.

## Next Steps

To implement this proof of concept:

1. Add the pool implementation to the Rust code
2. Add FFI exports for pool functions
3. Update the Kotlin bindings to expose the pool functions
4. Create utility classes for Kotlin developers
5. Write example code and documentation

This approach should provide significant performance improvements for high-throughput scenarios while maintaining backward compatibility with existing code.