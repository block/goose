import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*

fun main() = runBlocking {
    val now = System.currentTimeMillis() / 1000
    val msgs = listOf(
        // 1) User sends a plain-text prompt
        Message(
            role    = Role.USER,
            created = now,
            content = listOf(
                MessageContent.Text(
                    TextContent("What is 7 x 6?")
                )
            )
        ),

        // 2) Assistant makes a tool request (ToolReq) to calculate 7×6
        Message(
            role    = Role.ASSISTANT,
            created = now + 2,
            content = listOf(
                MessageContent.ToolReq(
                    ToolRequest(
                        id = "calc1",
                        toolCall = """
                            {
                              "status": "success",
                              "value": {
                                "name": "calculator_extension__toolname",
                                "arguments": {
                                  "operation": "multiply",
                                  "numbers": [7, 6]
                                }, 
                                "needsApproval": false
                              }                              
                            }
                        """.trimIndent()
                    )
                )
            )
        ),

        // 3) User (on behalf of the tool) responds with the tool result (ToolResp)
        Message(
            role    = Role.USER,
            created = now + 3,
            content = listOf(
                MessageContent.ToolResp(
                    ToolResponse(
                        id = "calc1",
                        toolResult = """
                            {
                              "status": "success",
                              "value": [
                                {"type": "text", "text": "42"}
                              ]                        
                            }
                        """.trimIndent()
                    )
                )
            )
        ), 
    )

    printMessages(msgs)
    println("---\n")

    // Setup provider
    val providerName = "databricks"
    val host = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerConfig = """{"host": "$host", "token": "$token"}"""

    println("Provider Name: $providerName")
    println("Provider Config: $providerConfig")


    val sessionName = generateSessionName(providerName, providerConfig, msgs)
    println("\nSession Name: $sessionName")

    val tooltip = generateTooltip(providerName, providerConfig, msgs)
    println("\nTooltip: $tooltip")

    // Completion
    val modelName = "goose-gpt-4-1"
    val modelConfig = ModelConfig(
        modelName,
        100000u,  // UInt
        0.1f,     // Float
        200      // Int
    )

    val calculatorTool = createToolConfig(
        name = "calculator",
        description = "Perform basic arithmetic operations",
        inputSchema = """
            {
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
            }
        """.trimIndent(),
        approvalMode = ToolApprovalMode.AUTO
    )

    val calculator_extension = ExtensionConfig(
        name = "calculator_extension",
        instructions = "This extension provides a calculator tool.",
        tools = listOf(calculatorTool)
    )

    val extensions = listOf(calculator_extension)
    val systemPreamble = "You are a helpful assistant."


    val req = createCompletionRequest(
        providerName,
        providerConfig,
        modelConfig,
        systemPreamble,
        msgs,
        extensions
    )

    val response = completion(req)
    println("\nCompletion Response:")
    println(response.message)
}
