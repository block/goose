import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*
import kotlin.random.Random

fun main() = runBlocking {
    // Setup provider config
    val providerName = System.getenv("PROVIDER_NAME") ?: "databricks"
    val host = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerConfig = """{"host": "$host", "token": "$token"}"""
    
    val modelName = System.getenv("MODEL_NAME") ?: "goose-gpt-4-1"
    val modelConfig = ModelConfig(
        modelName,
        100000u,  // context limit
        0.1f,     // temperature
        200      // max tokens
    )

    // Configure the provider pool
    println("Initializing and configuring provider pool...")
    ProviderPool.initialize()
    ProviderPool.configure(
        maxSize = 5,
        maxIdleSec = 300,
        maxLifetimeSec = 3600,
        maxUses = 50
    )
    
    println("Running provider pool performance test...")
    val tester = PoolPerformanceTester(
        providerName = providerName,
        providerConfig = providerConfig,
        modelConfig = modelConfig
    )
    
    // Run a benchmark with simple questions
    val result = tester.runBenchmark(
        iterations = 10,  // Run 10 iterations
        parallelism = 3,  // 3 parallel requests
        messageSupplier = {
            // Generate random test messages
            val question = getRandomQuestion()
            listOf(
                Message(
                    role = Role.USER,
                    created = System.currentTimeMillis() / 1000,
                    content = listOf(
                        MessageContent.Text(TextContent(question))
                    )
                )
            )
        }
    )

    // Print benchmark results
    println("\nPerformance Benchmark Results")
    println("============================")
    println(result)
    
    // Live agent example (interactive loop)
    println("\nWould you like to try an interactive demo? (y/n)")
    val input = readlnOrNull()
    if (input?.lowercase()?.startsWith("y") == true) {
        runInteractiveDemo(providerName, providerConfig, modelConfig)
    }
}

fun runInteractiveDemo(providerName: String, providerConfig: String, modelConfig: ModelConfig) = runBlocking {
    println("\nStarting interactive demo with connection pooling...\n")
    
    val service = ParallelCompletionService(
        providerName = providerName,
        providerConfig = providerConfig,
        modelConfig = modelConfig,
        usePool = true
    )
    
    val messages = mutableListOf<Message>()
    
    while (true) {
        print("\nYou (or 'exit' to quit): ")
        val userInput = readlnOrNull() ?: continue
        
        if (userInput.lowercase() == "exit") break
        
        // Add user message
        messages.add(
            Message(
                role = Role.USER,
                created = System.currentTimeMillis() / 1000,
                content = listOf(MessageContent.Text(TextContent(userInput)))
            )
        )
        
        // Process with connection pooling
        try {
            print("Assistant: ")
            val response = service.process(messages)
            
            // Add assistant response to conversation history
            messages.add(response.message)
            
            // Extract text from message content
            val responseText = response.message.content
                .filterIsInstance<MessageContent.Text>()
                .joinToString("\n") { it.v1.text }
            
            println(responseText)
            
            // Print some performance metrics
            println("\n[Processing time: ${response.runtimeMetrics.totalTimeSec}s]")
        } catch (e: Exception) {
            println("Error: ${e.message}")
        }
    }
    
    // Print final pool stats
    println("\nFinal provider pool statistics:")
    println(ProviderPool.stats())
}

// List of random questions for the benchmark
fun getRandomQuestion(): String {
    val questions = listOf(
        "What is the capital of France?",
        "How does photosynthesis work?",
        "Explain the basics of quantum computing",
        "What are the main causes of climate change?",
        "Who wrote the novel Pride and Prejudice?",
        "What is the difference between machine learning and AI?",
        "How do vaccines work?",
        "What are black holes?",
        "Explain the theory of relativity",
        "What is the Pythagorean theorem?",
        "How does the internet work?",
        "What is blockchain technology?",
        "What's the difference between HTTP and HTTPS?",
        "How does GPS navigation work?",
        "What is the history of the Olympic Games?"
    )
    return questions[Random.nextInt(questions.size)]
}