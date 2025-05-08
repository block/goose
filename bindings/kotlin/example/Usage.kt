import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*

fun main() = runBlocking {
    val msgs = listOf(
        Message(
            role    = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("Hello, how are you?")))
        ),
        Message(
            role    = Role.ASSISTANT,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("I’m fine, thanks! How can I help?")))
        ), 
        Message(
            role    = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("Why is the sky blue? Tell me in less than 20 words.")))
        ),
    )

    printMessages(msgs)
    println("---\n")

    val sessionName = generateSessionName(msgs)
    println("Session Name: $sessionName")

    val tooltip = generateTooltip(msgs)
    println("Tooltip: $tooltip")

    // Completion
    val provider = "databricks"
    val modelName = "goose-gpt-4-1"
    val modelConfig = ModelConfig(
        modelName,
        100000u,  // UInt
        0.1f,     // Float
        200      // Int
    )

    val calculatorToolJson = """
    {
        "name": "calculator",
        "description": "Perform basic arithmetic operations",
        "input_schema": {
            "type": "object",
            "required": ["operation", "numbers"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "numbers": {
                    "type": "array",
                    "items": { "type": "number" },
                    "description": "List of numbers to operate on in order"
                }
            }
        },
        "approval_mode": "Auto"
    }
    """.trimIndent()

    val extension = ExtensionConfig(
        "calculator_extension",
        "This extension provides a calculator tool.",
        listOf(calculatorToolJson)
    )

    val systemPreamble = "You are a helpful assistant."

    val messages = listOf(
        Message(
            role = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("Add 10037 + 23123 using calculator")))
        )
    )

    val req = CompletionRequest(
        provider,
        modelConfig,
        systemPreamble,
        messages,
        listOf(extension)
    )

    val response = completion(req)
    println("\nCompletion Response:")
    println(response.message)
}