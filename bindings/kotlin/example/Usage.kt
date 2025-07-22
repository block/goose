import java.io.File
import java.util.Base64
import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*

/* ----------  shared helpers ---------- */

fun buildProviderConfig(host: String, token: String, imageFormat: String = "OpenAi"): String = """
{
  "host": "$host",
  "token": "$token",
  "image_format": "$imageFormat"
}
""".trimIndent()

fun calculatorExtension(): ExtensionConfig {
    val calculatorTool = createToolConfig(
        name        = "calculator",
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
    return ExtensionConfig(
        name         = "calculator_extension",
        instructions = "This extension provides a calculator tool.",
        tools        = listOf(calculatorTool)
    )
}

/* ----------  demos ---------- */

suspend fun runCalculatorDemo(
    modelConfig: ModelConfig,
    providerName: String,
    providerConfig: String
) {
    val now  = System.currentTimeMillis() / 1000
    val msgs = listOf(
        // same conversation you already had
        Message(Role.USER,      now,     listOf(MessageContent.Text(TextContent("What is 7 x 6?")))),
        Message(Role.ASSISTANT, now + 2, listOf(MessageContent.ToolReq(
            ToolRequest(
                id       = "calc1",
                toolCall = """
                    {
                      "status": "success",
                      "value": {
                        "name": "calculator_extension__toolname",
                        "arguments": { "operation": "doesnotexist", "numbers": [7,6] },
                        "needsApproval": false
                      }
                    }
                """.trimIndent()
            )))),
        Message(Role.USER,      now + 3, listOf(MessageContent.ToolResp(
            ToolResponse(
                id         = "calc1",
                toolResult = """
                    {
                      "status": "error",
                      "error": "Invalid value for operation: 'doesnotexist'. Valid values are: ['add','subtract','multiply','divide']"
                    }
                """.trimIndent()
            )))),
        Message(Role.ASSISTANT, now + 4, listOf(MessageContent.ToolReq(
            ToolRequest(
                id       = "calc1",
                toolCall = """
                    {
                      "status": "success",
                      "value": {
                        "name": "calculator_extension__toolname",
                        "arguments": { "operation": "multiply", "numbers": [7,6] },
                        "needsApproval": false
                      }
                    }
                """.trimIndent()
            )))),
        Message(Role.USER,      now + 5, listOf(MessageContent.ToolResp(
            ToolResponse(
                id         = "calc1",
                toolResult = """
                    {
                      "status": "success",
                      "value": [ { "type": "text", "text": "42" } ]
                    }
                """.trimIndent()
            ))))
    )

    /* one-shot prompt with error  */
    val reqErr = createCompletionRequest(
        providerName, providerConfig, modelConfig,
        "You are a helpful assistant.",
        messages   = listOf(msgs.first()),
        extensions = listOf(calculatorExtension())
    )
    println("\n[${modelConfig.modelName}] Calculator (single-msg) → ${completion(reqErr).message}")

    /* full conversation */
    val reqAll = createCompletionRequest(
        providerName, providerConfig, modelConfig,
        "You are a helpful assistant.",
        messages   = msgs,
        extensions = listOf(calculatorExtension())
    )
    println("[${modelConfig.modelName}] Calculator (full chat)  → ${completion(reqAll).message}")
}

suspend fun runImageExample(
    modelConfig: ModelConfig,
    providerName: String,
    providerConfig: String
) {
    val imagePath   = "../../crates/goose/examples/test_assets/test_image.png"
    val base64Image = Base64.getEncoder().encodeToString(File(imagePath).readBytes())
    val now         = System.currentTimeMillis() / 1000

    val msgs = listOf(
        Message(Role.USER, now, listOf(
                MessageContent.Text(TextContent("What is in this image?")), 
                MessageContent.Image(ImageContent(base64Image, "image/png"))
        )),
    )

    val req = createCompletionRequest(
        providerName, providerConfig, modelConfig,
        "You are a helpful assistant. Please describe any text you see in the image.",
        messages = msgs, 
        extensions = emptyList()
    )

    println("\n[${modelConfig.modelName}] Image example → ${completion(req).message}")
}

suspend fun runPromptOverride(
    modelConfig: ModelConfig,
    providerName: String,
    providerConfig: String
) {
    val now  = System.currentTimeMillis() / 1000
    val req  = createCompletionRequest(
        providerName, providerConfig, modelConfig,
        systemPreamble       = null,
        systemPromptOverride = "You are a bot named Tile Creator. Your task is to create a tile based on the user's input.",
        messages             = listOf(
            Message(Role.USER, now, listOf(MessageContent.Text(TextContent("What's your name?"))))
        ),
        extensions           = emptyList()
    )
    println("\n[${modelConfig.modelName}] Prompt override → ${completion(req).message}")
}

suspend fun runUiExtraction(providerName: String, providerConfig: String) {
    val schema = /* same JSON schema as before */ """
        {
          "type":"object",
          "properties":{
            "type":{"type":"string","enum":["div","button","header","section","field","form"]},
            "label":{"type":"string"},
            "children":{"type":"array","items":{"${'$'}ref":"#"}},
            "attributes":{"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"],"additionalProperties":false}}
          },
          "required":["type","label","children","attributes"],
          "additionalProperties":false
        }
    """.trimIndent()

    val messages = listOf(
        Message(Role.USER, System.currentTimeMillis()/1000,
                listOf(MessageContent.Text(TextContent("Make a User Profile Form"))))
    )

    val res = generateStructuredOutputs(
        providerName, providerConfig,
        systemPrompt = "You are a UI generator AI. Convert the user input into a JSON-driven UI.",
        messages     = messages,
        schema       = schema
    )
    println("\n[UI-Extraction] → $res")
}

/* ----------  entry-point ---------- */

fun main() = runBlocking {
    /* --- provider setup --- */
    val providerName = "databricks"
    val host         = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token        = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerConfig = buildProviderConfig(host, token)

    println("Provider: $providerName")
    println("Config  : $providerConfig\n")

    /* --- run demos for each model --- */
    // NOTE: `claude-3-5-haiku` does NOT support images 
    val modelNames = listOf("kgoose-gpt-4o", "goose-claude-4-sonnet")

    for (name in modelNames) {
        val modelConfig = ModelConfig(name, 100000u, 0.1f, 200)
        println("\n=====  Running demos for model: $name  =====")

        runCalculatorDemo(modelConfig, providerName, providerConfig)
        runImageExample(modelConfig,    providerName, providerConfig)
        runPromptOverride(modelConfig,  providerName, providerConfig)
        println("=====  End demos for $name  =====\n")
    }

    /* UI extraction is model-agnostic, so run it once */
    runUiExtraction(providerName, providerConfig)
}
