import kotlinx.coroutines.runBlocking
import uniffi.goose_uniffi.*

fun main() = runBlocking {
    val now = System.currentTimeMillis() / 1000
    val msgs = listOf(
        // 1) User sends a plain-text prompt
        Message(
            role    = Role.USER,
            created = now,
            content = listOf(
                MessageContent.Text(
                    TextContent("What is 7 × 6?")
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
                                "params": {
                                  "operation": "multiply",
                                  "numbers": [7, 6]
                                }
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
                              "value": [{
                                "text": "7 x 6 = 42"
                              }]                        
                            }
                        """.trimIndent()
                    )
                )
            )
        ),

        // 4) Assistant follows up in plain text
        Message(
            role    = Role.ASSISTANT,
            created = now + 4,
            content = listOf(
                MessageContent.Text(
                    TextContent("The answer is 42.")
                )
            )
        )
    )

    printMessages(msgs)
    println("---\n")

    val calculatorTool = createToolConfig(
        name = "calculator",
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
    )

    val extensions = listOf(
        ExtensionConfig(
            "calculator_extension",
            listOf(calculatorTool)
        )
    )

    asyncPrint(msgs, extensions)
}
