import kotlinx.coroutines.*
import uniffi.goose_llm.*
import kotlin.system.measureNanoTime

/**
 * Simple example showing connection pooling in action.
 * This example avoids the experimental/advanced features and just shows basic pooling.
 */
fun main() = runBlocking {
    // Get provider details from environment variables
    val providerName = System.getenv("PROVIDER_NAME") ?: "databricks"
    val host = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerConfig = """{"host": "$host", "token": "$token"}"""
    
    val modelName = System.getenv("MODEL_NAME") ?: "goose-gpt-4-1" 
    val modelConfig = ModelConfig(
        modelName = modelName,
        contextLimit = 100000u,
        temperature = 0.7f,
        maxTokens = 1000
    )

    println("Initializing connection pool...")
    // Initialize the provider pool with default settings
    try {
        initProviderPool()
    } catch (e: Exception) {
        println("Warning: Pool initialization failed: ${e.message}")
        println("Proceeding without connection pooling")
    }

    // Set up a simple question
    val question = "What is connection pooling and why is it useful?"
    val messages = listOf(
        Message(
            role = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent(question)))
        )
    )

    // First call without pooling for comparison
    println("\nMaking request WITHOUT connection pooling...")
    var duration1 = 0L
    val request1 = createCompletionRequest(
        providerName = providerName,
        providerConfig = providerConfig,
        modelConfig = modelConfig,
        systemPreamble = "You are a helpful assistant.",
        messages = messages,
        extensions = emptyList(),
        usePool = false  // Disable pooling for this request
    )

    // Process the completion
    try {
        var response1: CompletionResponse
        duration1 = measureNanoTime {
            response1 = completion(request1)
        }
        
        // Extract and print response
        val text1 = response1.message.content
            .filterIsInstance<MessageContent.Text>()
            .joinToString { it.v1.text }
            
        println("Response: ${text1.take(150)}...")
        println("Time taken: ${duration1 / 1_000_000}ms")
    } catch (e: Exception) {
        println("Error with non-pooled request: ${e.message}")
    }

    // Second call with pooling
    println("\nMaking request WITH connection pooling...")
    var duration2 = 0L
    val request2 = createCompletionRequest(
        providerName = providerName,
        providerConfig = providerConfig,
        modelConfig = modelConfig,
        systemPreamble = "You are a helpful assistant.",
        messages = messages,
        extensions = emptyList(),
        usePool = true  // Enable pooling for this request
    )

    // Process the completion
    try {
        var response2: CompletionResponse
        duration2 = measureNanoTime {
            response2 = completion(request2)
        }
        
        // Extract and print response
        val text2 = response2.message.content
            .filterIsInstance<MessageContent.Text>()
            .joinToString { it.v1.text }
            
        println("Response: ${text2.take(150)}...")
        println("Time taken: ${duration2 / 1_000_000}ms")
        
        // Calculate speedup
        if (duration1 > 0) {
            val speedup = (duration1.toDouble() / duration2.toDouble()) - 1.0
            println("Speedup with connection pooling: ${String.format("%.2f", speedup * 100)}%")
        }
        
        // Show pool stats
        println("\nProvider Pool Statistics:")
        println(getPoolStats())
    } catch (e: Exception) {
        println("Error with pooled request: ${e.message}")
    }
    
    println("\nDone!")
}