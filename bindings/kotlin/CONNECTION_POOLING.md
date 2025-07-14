# Connection Pooling for Goose LLM in Kotlin

This document explains how to use connection pooling with goose-llm in your Kotlin application to improve performance when making many parallel requests.

## Overview

Connection pooling reuses provider connections instead of creating a new one for each request. This provides significant performance improvements when:

1. Making many parallel requests
2. Processing numerous sequential requests
3. Handling high-throughput agent loops

## Important Note

The current implementation has been updated for type safety for compatibility with Rust's UniFFI bindings. If you encounter any issues, please check that:

1. You're using the latest version of the Kotlin bindings
2. The values passed to `configureProviderPool` are within appropriate ranges for `u32` types

## Basic Usage

### 1. Initialize and Configure the Pool

Initialize the provider pool at application startup:

```kotlin
import uniffi.goose_llm.*

// Initialize with default settings (10 max connections, 5 min idle timeout)
initProviderPool()

// Or with custom configuration
configureProviderPool(
    maxSize = 20,            // Maximum number of connections in the pool
    maxIdleSec = 300,        // Maximum idle time (seconds) before cleanup
    maxLifetimeSec = 3600,   // Maximum lifetime (seconds) for a connection
    maxUses = 100            // Maximum number of uses before recycling
)
```

### 2. Create Completion Requests with Pool Option

```kotlin
val request = createCompletionRequest(
    providerName = "databricks",
    providerConfig = providerConfig,
    modelConfig = modelConfig,
    systemPreamble = "You are a helpful assistant.",
    messages = messages,
    extensions = extensions,
    usePool = true  // Enable connection pooling (default is true)
)

// Process the completion
val response = completion(request)
```

### 3. Monitor Pool Statistics

```kotlin
// Get pool statistics as a string
val stats = getPoolStats()
println(stats)

// Example output:
// Pool: openai:d41d8cd98f00b204e9800998ecf8427e:gpt-4o
//   Created: 5
//   Borrowed: 15
//   Returned: 15
//   Errors: 0
//   Max Pool Size: 10
//   Current Pool Size: 5
//   Waiting: 0
```

## Advanced Usage

### Parallel Completion Service

The `ParallelCompletionService` class provides a high-level wrapper for processing multiple messages in parallel with connection pooling:

```kotlin
val service = ParallelCompletionService(
    providerName = "databricks",
    providerConfig = providerConfig,
    modelConfig = modelConfig,
    maxConcurrency = 5,
    usePool = true
)

// Process multiple message lists in parallel
val responses = service.processInParallel(messageListsToProcess)
```

### Agent Service

For production agent services, use the `AgentService` class:

```kotlin
val agentService = AgentService(
    providerName = "databricks",
    providerConfig = providerConfig,
    modelName = "goose-gpt-4-1",
    maxConcurrentAgents = 10,
    useConnectionPooling = true
)

// Process agent requests in parallel
val responses = agentService.processMessagesInParallel(requests)

// Get service metrics
val metrics = agentService.getMetrics()
```

## Performance Benchmarking

Use the `PoolPerformanceTester` to benchmark your specific workload:

```kotlin
val tester = PoolPerformanceTester(
    providerName = providerName,
    providerConfig = providerConfig,
    modelConfig = modelConfig
)

val result = tester.runBenchmark(
    iterations = 10,    // Number of requests to make
    parallelism = 5,    // Max parallel requests
    messageSupplier = { /* create test messages */ }
)

println(result)
```

## Connection Pool Configuration Recommendations

| Scenario | Max Size | Idle Timeout | Lifetime | Max Uses |
|----------|----------|--------------|----------|----------|
| Low volume | 5 | 300s (5min) | 3600s (1hr) | 50 |
| Medium volume | 10-20 | 600s (10min) | 7200s (2hr) | 100 |
| High volume | 30-50 | 900s (15min) | 10800s (3hr) | 200 |
| Batch processing | 50-100 | 300s (5min) | 3600s (1hr) | 50 |

## Troubleshooting

If you encounter issues:

1. Check pool statistics to diagnose connection usage
2. Ensure the pool size is appropriate for your concurrency level
3. Try increasing timeout values if connections are being recycled too often
4. Fall back to non-pooled connections by setting `usePool = false` to compare behavior

## Notes and Limitations

- The pool uses separate connection groups per provider config and model
- Connection errors are handled by creating new connections
- Provider maintenance is automatic with periodic cleanup of idle connections
- Metrics are available via `getPoolStats()` for monitoring

For more examples, see:
- `AgentService.kt` - Production service with connection pooling
- `ProviderPool.kt` - Kotlin wrapper for the provider pool
- `PoolingDemo.kt` - Interactive demo and benchmark