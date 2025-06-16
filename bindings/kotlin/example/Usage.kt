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
                                  "operation": "doesnotexist",
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
                              "status": "error",
                              "error": "Invalid value for operation: 'doesnotexist'. Valid values are: ['add', 'subtract', 'multiply', 'divide']"
                            }
                        """.trimIndent()
                    )
                )
            )
        ), 

        // 4) Assistant makes a tool request (ToolReq) to calculate 7×6
        Message(
            role    = Role.ASSISTANT,
            created = now + 4,
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

        // 5) User (on behalf of the tool) responds with the tool result (ToolResp)
        Message(
            role    = Role.USER,
            created = now + 5,
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

    // Testing with tool calls with an error in tool name
    val reqToolErr = createCompletionRequest(
        providerName,
        providerConfig,
        modelConfig,
        systemPreamble,
        messages = listOf(
            Message(
                role    = Role.USER,
                created = now,
                content = listOf(
                    MessageContent.Text(
                        TextContent("What is 7 x 6?")
                    )
                )
            )),
        extensions = extensions
    )

    val respToolErr = completion(reqToolErr)
    println("\nCompletion Response (one msg):\n${respToolErr.message}")
    println()

    val reqAll = createCompletionRequest(
        providerName,
        providerConfig,
        modelConfig,
        systemPreamble,
        messages = msgs,
        extensions = extensions
    )

    val respAll = completion(reqAll)
    println("\nCompletion Response (all msgs):\n${respAll.message}")
    println()

    // ---- UI Extraction (custom schema) ----
    runUiExtraction(providerName, providerConfig)

    // --- Prompt Override ---
    val prompt_req = createCompletionRequest(
        providerName,
        providerConfig,
        modelConfig,
        systemPreamble = null, 
        systemPromptOverride = "You are a bot named Tile Creator. Your task is to create a tile based on the user's input.",
        messages=listOf(
            Message(
                role    = Role.USER,
                created = now,
                content = listOf(
                    MessageContent.Text(
                        TextContent("What's your name?")
                    )
                )
            )
        ),
        extensions=emptyList()
    )

    val prompt_resp = completion(prompt_req)

    println("\nPrompt Override Response:\n${prompt_resp.message}")
}


suspend fun runUiExtraction(providerName: String, providerConfig: String) {
    val systemPrompt = "You are a UI generator AI. Convert the user input into a JSON-driven UI."
    val messages = listOf(
        Message(
            role = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(
                MessageContent.Text(
                    TextContent("Make a User Profile Form")
                )
            )
        )
    )
    val schema = """{
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["div","button","header","section","field","form"]
            },
            "label":   { "type": "string" },
            "children": {
                "type": "array",
                "items": { "${'$'}ref": "#" }
            },
            "attributes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name":  { "type": "string" },
                        "value": { "type": "string" }
                    },
                    "required": ["name","value"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["type","label","children","attributes"],
        "additionalProperties": false
    }""".trimIndent();

    try {
        val response = generateStructuredOutputs(
            providerName = providerName,
            providerConfig = providerConfig,
            systemPrompt = systemPrompt,
            messages = messages,
            schema = schema
        )
        println("\nUI Extraction Output:\n${response}")
    } catch (e: ProviderException) {
        println("\nUI Extraction failed:\n${e.message}")
    }
}
