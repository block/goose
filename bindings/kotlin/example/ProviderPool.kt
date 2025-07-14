import kotlinx.coroutines.*
import uniffi.goose_llm.*
import kotlin.system.measureTimeMillis

/**
 * Provider pool for managing and optimizing LLM provider connections
 *
 * This class wraps the underlying Rust provider pool system to make it easy
 * to use from Kotlin. It provides methods to configure and monitor the pool.
 * 
 * Note: The connection pool settings are passed to the Rust layer as u32 types,
 * so very large values (> 2^31-1) will be clamped.
 */
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
         *
         * @param maxSize The maximum number of connections in the pool
         * @param maxIdleSec Maximum time a connection can be idle before cleanup
         * @param maxLifetimeSec Maximum lifetime of a connection
         * @param maxUses Maximum number of uses for a connection
         */
        fun configure(maxSize: Int = 10, maxIdleSec: Long = 300, 
                      maxLifetimeSec: Long = 3600, maxUses: Int = 100) {
            configureProviderPool(maxSize, maxIdleSec, maxLifetimeSec, maxUses)
        }

        /**
         * Get statistics about the current connection pool
         *
         * @return A string containing the pool statistics
         */
        fun stats(): String {
            return getPoolStats()
        }
    }
}

/**
 * Extension function to create a completion request with pool options
 *
 * @param usePool Whether to use the connection pool
 */
fun createPooledCompletionRequest(
    providerName: String,
    providerConfig: String,
    modelConfig: ModelConfig,
    systemPreamble: String,
    messages: List<Message>,
    extensions: List<ExtensionConfig>,
    usePool: Boolean = true
): CompletionRequest {
    return createCompletionRequest(
        providerName,
        providerConfig,
        modelConfig,
        systemPreamble,
        messages,
        extensions,
        usePool
    )
}

/**
 * Parallel completion service for handling multiple requests efficiently
 *
 * This class helps manage multiple parallel completion requests with
 * connection pooling for optimal performance.
 */
class ParallelCompletionService(
    private val providerName: String,
    private val providerConfig: String,
    private val modelConfig: ModelConfig,
    private val systemPreamble: String = "You are a helpful assistant.",
    private val extensions: List<ExtensionConfig> = emptyList(),
    private val maxConcurrency: Int = 5,
    private val usePool: Boolean = true
) {
    private val dispatcher = Dispatchers.IO.limitedParallelism(maxConcurrency)

    init {
        // Initialize the provider pool
        ProviderPool.initialize()
    }

    /**
     * Process multiple messages in parallel
     *
     * @param messages List of message lists to process
     * @return List of completion responses
     */
    suspend fun processInParallel(messages: List<List<Message>>): List<CompletionResponse> = 
        coroutineScope {
            messages.map { messageList ->
                async(dispatcher) {
                    val request = createPooledCompletionRequest(
                        providerName,
                        providerConfig,
                        modelConfig,
                        systemPreamble,
                        messageList,
                        extensions,
                        usePool
                    )
                    completion(request)
                }
            }.awaitAll()
        }
    
    /**
     * Process a single message list using the service configuration
     *
     * @param messages Message list to process
     * @return Completion response
     */
    suspend fun process(messages: List<Message>): CompletionResponse {
        val request = createPooledCompletionRequest(
            providerName,
            providerConfig,
            modelConfig,
            systemPreamble,
            messages,
            extensions,
            usePool
        )
        return completion(request)
    }
}

/**
 * Performance testing functions for comparing pooled vs non-pooled performance
 */
class PoolPerformanceTester(
    private val providerName: String,
    private val providerConfig: String,
    private val modelConfig: ModelConfig,
    private val systemPreamble: String = "You are a helpful assistant."
) {
    // Configure the pool with default settings
    init {
        ProviderPool.initialize()
        ProviderPool.configure()
    }

    /**
     * Run a performance benchmark comparing pooled vs non-pooled completions
     * 
     * @param iterations Number of iterations to perform
     * @param parallelism Maximum parallel requests
     * @param messageSupplier Function to generate test messages
     * @return Benchmark results
     */
    suspend fun runBenchmark(
        iterations: Int = 10,
        parallelism: Int = 3,
        messageSupplier: () -> List<Message>
    ): BenchmarkResult = coroutineScope {
        // Run test with pooling
        val poolTimeMs = measureTimeMillis {
            processRequests(iterations, parallelism, true, messageSupplier)
        }

        delay(1000) // Give pool time to stabilize between tests
        
        // Run test without pooling
        val nonPoolTimeMs = measureTimeMillis {
            processRequests(iterations, parallelism, false, messageSupplier)
        }

        // Calculate improvement percentage
        val improvementPercent = if (nonPoolTimeMs > 0) {
            ((nonPoolTimeMs - poolTimeMs) * 100.0) / nonPoolTimeMs
        } else 0.0

        BenchmarkResult(
            iterations = iterations,
            parallelism = parallelism,
            pooledTimeMs = poolTimeMs,
            nonPooledTimeMs = nonPoolTimeMs,
            improvementPercent = improvementPercent,
            poolStats = ProviderPool.stats()
        )
    }

    private suspend fun processRequests(
        iterations: Int,
        parallelism: Int,
        usePool: Boolean,
        messageSupplier: () -> List<Message>
    ) = coroutineScope {
        val service = ParallelCompletionService(
            providerName = providerName,
            providerConfig = providerConfig,
            modelConfig = modelConfig,
            systemPreamble = systemPreamble,
            maxConcurrency = parallelism,
            usePool = usePool
        )

        val messages = (1..iterations).map { messageSupplier() }
        service.processInParallel(messages)
    }
}

/**
 * Results from a pooling performance benchmark
 */
data class BenchmarkResult(
    val iterations: Int,
    val parallelism: Int,
    val pooledTimeMs: Long,
    val nonPooledTimeMs: Long,
    val improvementPercent: Double,
    val poolStats: String
) {
    override fun toString(): String = buildString {
        append("Benchmark Results:\n")
        append("------------------\n")
        append("Iterations: $iterations\n")
        append("Parallelism: $parallelism\n")
        append("Pooled time: ${pooledTimeMs}ms (${pooledTimeMs / iterations}ms per request)\n")
        append("Non-pooled time: ${nonPooledTimeMs}ms (${nonPooledTimeMs / iterations}ms per request)\n")
        append("Improvement: ${String.format("%.2f", improvementPercent)}%\n")
        append("\nPool Statistics:\n")
        append(poolStats)
    }
}