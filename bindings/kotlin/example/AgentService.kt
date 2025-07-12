import kotlinx.coroutines.*
import uniffi.goose_llm.*
import java.time.Instant
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlin.time.Duration.Companion.seconds

/**
 * Production-ready agent service with connection pooling
 *
 * This service is designed for high-throughput parallel agent processing
 * using the connection pool for optimal performance.
 */
class AgentService(
    private val providerName: String,
    private val providerConfig: String,
    private val modelName: String,
    private val maxConcurrentAgents: Int = 20,
    private val useConnectionPooling: Boolean = true
) {
    // Service state
    private val sessionMap = ConcurrentHashMap<String, AgentSession>()
    private val activeRequests = AtomicInteger(0)
    private val completionScope = CoroutineScope(Dispatchers.Default)
    
    // Performance metrics
    private val totalRequests = AtomicInteger(0)
    private val totalErrors = AtomicInteger(0)
    private val totalResponseTimeMs = AtomicInteger(0)
    
    // Initialize with custom pool settings
    init {
        // Configure connection pool for optimal parallelism
        ProviderPool.initialize()
        ProviderPool.configure(
            maxSize = maxConcurrentAgents,
            maxIdleSec = 300,   // 5 minutes idle timeout
            maxLifetimeSec = 1800,  // 30 minutes lifetime
            maxUses = 100  // Max uses per connection
        )
    }
    
    /**
     * Process an agent message in a specific session
     */
    suspend fun processMessage(sessionId: String, userMessage: String): AgentResponse = withContext(Dispatchers.IO) {
        val startTime = System.currentTimeMillis()
        activeRequests.incrementAndGet()
        
        try {
            totalRequests.incrementAndGet()
            
            // Get or create session
            val session = sessionMap.computeIfAbsent(sessionId) { 
                AgentSession(sessionId)
            }
            
            // Process the message
            val response = session.addUserMessage(userMessage)
                .processWithPooling(useConnectionPooling)
            
            // Update metrics
            val elapsed = System.currentTimeMillis() - startTime
            totalResponseTimeMs.addAndGet(elapsed.toInt())
            
            response
        } catch (e: Exception) {
            totalErrors.incrementAndGet()
            AgentResponse(
                text = "Error: ${e.message}",
                processingTimeMs = System.currentTimeMillis() - startTime,
                error = true
            )
        } finally {
            activeRequests.decrementAndGet()
        }
    }
    
    /**
     * Process multiple agent messages in parallel
     */
    suspend fun processMessagesInParallel(requests: List<AgentRequest>): List<AgentResponse> = coroutineScope {
        requests.map { request ->
            async(Dispatchers.IO.limitedParallelism(maxConcurrentAgents)) {
                processMessage(request.sessionId, request.message)
            }
        }.awaitAll()
    }
    
    /**
     * Get current service metrics
     */
    fun getMetrics(): ServiceMetrics {
        val totalReq = totalRequests.get()
        return ServiceMetrics(
            totalRequests = totalReq,
            totalErrors = totalErrors.get(),
            activeRequests = activeRequests.get(),
            avgResponseTimeMs = if (totalReq > 0) totalResponseTimeMs.get() / totalReq else 0,
            poolStats = ProviderPool.stats()
        )
    }
    
    /**
     * Session handling for agent conversations
     */
    inner class AgentSession(val sessionId: String) {
        private val messages = mutableListOf<Message>()
        private val createdAt = Instant.now()
        
        fun addUserMessage(text: String): AgentSession {
            messages.add(
                Message(
                    role = Role.USER,
                    created = System.currentTimeMillis() / 1000,
                    content = listOf(MessageContent.Text(TextContent(text)))
                )
            )
            return this
        }
        
        suspend fun processWithPooling(usePool: Boolean): AgentResponse {
            val startTime = System.currentTimeMillis()
            
            // Create model config
            val modelConfig = ModelConfig(
                modelName = modelName,
                contextLimit = 100000u,
                temperature = 0.7f,
                maxTokens = 1000
            )
            
            // Create completion request (with or without pooling)
            val request = createPooledCompletionRequest(
                providerName = providerName,
                providerConfig = providerConfig,
                modelConfig = modelConfig,
                systemPreamble = "You are a helpful assistant.",
                messages = messages,
                extensions = emptyList(),
                usePool = usePool
            )
            
            // Process the completion
            val result = completion(request)
            
            // Add assistant response to session history
            messages.add(result.message)
            
            // Extract text content from the response
            val responseText = result.message.content
                .filterIsInstance<MessageContent.Text>()
                .joinToString("\n") { it.v1.text }
            
            return AgentResponse(
                text = responseText,
                processingTimeMs = System.currentTimeMillis() - startTime,
                error = false
            )
        }
    }
}

/**
 * Agent request for batch processing
 */
data class AgentRequest(
    val sessionId: String,
    val message: String
)

/**
 * Agent response with metrics
 */
data class AgentResponse(
    val text: String,
    val processingTimeMs: Long,
    val error: Boolean
)

/**
 * Service metrics for monitoring
 */
data class ServiceMetrics(
    val totalRequests: Int,
    val totalErrors: Int,
    val activeRequests: Int,
    val avgResponseTimeMs: Int,
    val poolStats: String
)

/**
 * Example usage of the AgentService
 */
suspend fun main() = coroutineScope {
    // Setup provider config
    val providerName = System.getenv("PROVIDER_NAME") ?: "databricks"
    val host = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerConfig = """{"host": "$host", "token": "$token"}"""
    val modelName = System.getenv("MODEL_NAME") ?: "goose-gpt-4-1"
    
    // Create agent service
    val agentService = AgentService(
        providerName = providerName,
        providerConfig = providerConfig,
        modelName = modelName,
        maxConcurrentAgents = 5,
        useConnectionPooling = true  // Enable connection pooling
    )
    
    // Process a few requests in parallel 
    val responses = agentService.processMessagesInParallel(listOf(
        AgentRequest("session1", "What is connection pooling?"),
        AgentRequest("session2", "Explain the benefits of async programming"),
        AgentRequest("session3", "How does Kotlin coroutines work?"),
        AgentRequest("session1", "How does it compare to thread pools?"),
        AgentRequest("session2", "What about error handling?")
    ))
    
    // Print responses
    responses.forEachIndexed { i, response ->
        println("Response ${i+1}:")
        println("Text: ${response.text.take(100)}...")
        println("Processing time: ${response.processingTimeMs}ms")
        println("Error: ${response.error}")
        println("-----")
    }
    
    // Print metrics
    val metrics = agentService.getMetrics()
    println("Service Metrics:")
    println("  Total requests: ${metrics.totalRequests}")
    println("  Total errors: ${metrics.totalErrors}")
    println("  Active requests: ${metrics.activeRequests}")
    println("  Average response time: ${metrics.avgResponseTimeMs}ms")
    println("\nPool Stats:")
    println(metrics.poolStats)
}