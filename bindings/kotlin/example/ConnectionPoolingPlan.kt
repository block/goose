import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*

/**
 * This example shows a minimal proof-of-concept for implementing connection pooling
 * with the existing code, without requiring Rust-side changes.
 */
fun main() = runBlocking {
    println("Connection Pooling for goose-llm")
    println("================================")
    println("This is a simple proof of concept for connection pooling.")
    println("Here's how we can implement connection pooling with minimal changes:")
    
    // Define our provider pool in Kotlin using a simple cache
    val providerCache = ProviderCache()
    
    println("\n1. Making parallel requests WITHOUT connection pooling...")
    val nonPooledResults = runParallelRequests(3, useCache = false)
    
    println("\n2. Making parallel requests WITH connection pooling...")
    val pooledResults = runParallelRequests(3, useCache = true)
    
    // Calculate and print statistics
    val avgNonPooled = nonPooledResults.average()
    val avgPooled = pooledResults.average()
    val improvement = (avgNonPooled - avgPooled) * 100 / avgNonPooled
    
    println("\nResults:")
    println("- Average time without pooling: ${String.format("%.2f", avgNonPooled)}ms")
    println("- Average time with pooling:    ${String.format("%.2f", avgPooled)}ms")
    println("- Performance improvement:      ${String.format("%.2f", improvement)}%")
    
    println("\nProvider Cache Statistics:")
    println("- Created:  ${providerCache.created}")
    println("- Retrieved: ${providerCache.retrieved}")
    println("- Current Size: ${providerCache.size()}")
}

/**
 * Run multiple parallel requests and record the timings
 */
suspend fun runParallelRequests(count: Int, useCache: Boolean): List<Long> = coroutineScope {
    // Create tasks
    val tasks = (1..count).map { id ->
        async {
            val startTime = System.currentTimeMillis()
            println("  Starting request $id...")
            
            // Make the request
            try {
                // In reality this would call the provider with completion
                simulateProviderUsage(useCache)
                
                val duration = System.currentTimeMillis() - startTime
                println("  Completed request $id in ${duration}ms")
                duration
            } catch (e: Exception) {
                println("  Request $id failed: ${e.message}")
                -1L
            }
        }
    }
    
    // Wait for all tasks and filter out errors
    tasks.awaitAll().filter { it > 0 }
}

/**
 * Simulate provider usage with or without a cache
 * Note: In a real implementation, this would use the actual LLM providers
 */
fun simulateProviderUsage(useCache: Boolean) {
    // Simulate provider creation and usage time
    val createTime = 500L // 500ms to create a provider
    val useTime = 1000L // 1000ms to use the provider
    
    if (useCache) {
        // With caching - only creation time for first use
        val provider = ProviderCache.get("sample-provider")
        Thread.sleep(useTime) // Simulate provider usage time
    } else {
        // Without caching - pay creation cost every time
        Thread.sleep(createTime) // Simulate provider creation time
        Thread.sleep(useTime) // Simulate provider usage time
    }
}

/**
 * A simple provider cache to simulate connection pooling
 */
class ProviderCache {
    companion object {
        private val cache = mutableMapOf<String, Any>()
        var created = 0
        var retrieved = 0
        
        fun get(key: String): Any {
            return if (cache.containsKey(key)) {
                retrieved++
                cache[key]!!
            } else {
                // Create new provider
                created++
                Thread.sleep(500) // Simulate creation time
                val provider = Any() // In reality this would be the provider
                cache[key] = provider
                provider
            }
        }
        
        fun size(): Int = cache.size
    }
}

/**
 * Implementation Plan for Real Connection Pooling
 *
 * 1. Rust Side:
 *    - Create a thread-safe provider pool using Arc<Mutex<HashMap<String, Vec<Arc<dyn Provider>>>>>
 *    - Add pool configuration settings (max size, idle timeout, etc.)
 *    - Export get_provider/return_provider functions via FFI
 *    - Add pool statistics
 *
 * 2. Kotlin Side:
 *    - Create wrapper classes to manage the provider pool
 *    - Add utility functions for pool management
 *    - Integrate with completion requests
 *
 * 3. Benefits:
 *    - Reduced latency for repeated requests
 *    - Better resource utilization
 *    - Improved performance for parallel requests
 *    - More stable connection handling
 *    - Configurable pool behavior
 */