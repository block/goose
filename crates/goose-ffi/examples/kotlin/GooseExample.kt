import com.sun.jna.*
import com.sun.jna.ptr.PointerByReference
import java.util.Arrays

// Define the JNA interface for the Goose FFI library
interface GooseFFI : Library {
    companion object {
        val INSTANCE: GooseFFI = Native.load("/Users/smohammed/Development/goose/target/debug/libgoose_ffi.dylib", GooseFFI::class.java)
    }

    // Enums
    object MessageRole {
        const val USER = 0
        const val ASSISTANT = 1
        const val SYSTEM = 2
    }

    object ProviderType {
        const val DATABRICKS = 0
    }

    // Structures
    class ProviderConfigFFI : Structure() {
        @JvmField var provider_type: Int = 0
        @JvmField var api_key: String? = null
        @JvmField var model_name: String? = null
        @JvmField var host: String? = null

        override fun getFieldOrder(): List<String> {
            return listOf("provider_type", "api_key", "model_name", "host")
        }
    }

    open class MessageFFI : Structure {
        @JvmField var role: Int = 0
        @JvmField var content: String? = null

        constructor() : super()
        constructor(p: Pointer) : super(p)

        override fun getFieldOrder(): List<String> {
            return listOf("role", "content")
        }

        class ByReference : MessageFFI(), Structure.ByReference
    }

    open class ToolFFI : Structure {
        @JvmField var name: String? = null
        @JvmField var description: String? = null
        @JvmField var input_schema_json: String? = null

        constructor() : super()
        constructor(p: Pointer) : super(p)

        override fun getFieldOrder(): List<String> {
            return listOf("name", "description", "input_schema_json")
        }

        class ByReference : ToolFFI(), Structure.ByReference
    }

    open class CompletionResponseFFI : Structure {
        @JvmField var content: Pointer? = null
        @JvmField var succeeded: Boolean = false
        @JvmField var error_message: Pointer? = null

        constructor() : super()
        constructor(p: Pointer) : super(p)

        override fun getFieldOrder(): List<String> {
            return listOf("content", "succeeded", "error_message")
        }

        class ByReference : CompletionResponseFFI(), Structure.ByReference
    }

    // Functions
    fun goose_agent_new(config: ProviderConfigFFI): Pointer?
    fun goose_agent_free(agent_ptr: Pointer?)
    fun goose_agent_send_message(agent_ptr: Pointer?, message: String): Pointer?
    fun goose_free_string(s: Pointer?)
    fun goose_completion(
        provider: String,
        model_name: String,
        host: String?,
        api_key: String?,
        system_preamble: String,
        messages_ptr: MessageFFI?,
        message_count: Long,
        tools_ptr: ToolFFI?,
        tool_count: Long,
        check_tool_approval: Boolean
    ): CompletionResponseFFI.ByReference?
    fun goose_free_completion_response(response: CompletionResponseFFI.ByReference?)
}

// Helper class to manage Goose interactions
class GooseClient {
    private val goose = GooseFFI.INSTANCE

    fun createCalculatorTool(): GooseFFI.ToolFFI {
        val tool = GooseFFI.ToolFFI()
        tool.name = "calculator"
        tool.description = "Perform basic arithmetic operations"
        tool.input_schema_json = """
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
                        "items": {"type": "number"},
                        "description": "List of numbers to operate on in order"
                    }
                }
            }
        """.trimIndent()
        return tool
    }

    fun createBashTool(): GooseFFI.ToolFFI {
        val tool = GooseFFI.ToolFFI()
        tool.name = "bash_shell"
        tool.description = "Run a shell command"
        tool.input_schema_json = """
            {
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                }
            }
        """.trimIndent()
        return tool
    }

    fun createMessage(role: Int, content: String): GooseFFI.MessageFFI {
        val message = GooseFFI.MessageFFI()
        message.role = role
        message.content = content
        return message
    }

    fun performCompletion(
        provider: String,
        modelName: String,
        systemPreamble: String,
        messages: List<GooseFFI.MessageFFI>,
        tools: List<GooseFFI.ToolFFI>,
        checkToolApproval: Boolean
    ): String {
        // Convert messages to array
        val messagesArray = if (messages.isNotEmpty()) {
            val array = GooseFFI.MessageFFI().toArray(messages.size) as Array<GooseFFI.MessageFFI>
            for (i in messages.indices) {
                array[i].role = messages[i].role
                array[i].content = messages[i].content
                array[i].write()
            }
            array[0]
        } else {
            null
        }

        // Convert tools to array
        val toolsArray = if (tools.isNotEmpty()) {
            val array = GooseFFI.ToolFFI().toArray(tools.size) as Array<GooseFFI.ToolFFI>
            for (i in tools.indices) {
                array[i].name = tools[i].name
                array[i].description = tools[i].description
                array[i].input_schema_json = tools[i].input_schema_json
                array[i].write()
            }
            array[0]
        } else {
            null
        }

        // Perform the completion
        val response = goose.goose_completion(
            provider,
            modelName,
            null,
            null,
            systemPreamble,
            messagesArray,
            messages.size.toLong(),
            toolsArray,
            tools.size.toLong(),
            checkToolApproval
        )

        try {
            if (response != null) {
                response.read()
                if (response.succeeded) {
                    return response.content?.getString(0) ?: "No content"
                } else {
                    val errorMsg = response.error_message?.getString(0) ?: "Unknown error"
                    throw RuntimeException("Completion failed: $errorMsg")
                }
            } else {
                throw RuntimeException("Completion returned null")
            }
        } finally {
            goose.goose_free_completion_response(response)
        }
    }
}

fun main() {
    val client = GooseClient()

    // Create tools
    val calculatorTool = client.createCalculatorTool()
    val bashTool = client.createBashTool()
    val tools = listOf(calculatorTool, bashTool)

    // Test with different prompts
    val prompts = listOf(
        "Add 10037 + 23123",
        "Write some random bad words to end of words.txt",
        "List all json files in the current directory and then multiply the count of the files by 7"
    )

    for (prompt in prompts) {
        println("\n${"=".repeat(50)}")
        println("User Input: $prompt")
        println("${"=".repeat(50)}")

        // Create messages
        val messages = listOf(
            client.createMessage(GooseFFI.MessageRole.USER, "hi there"),
            client.createMessage(GooseFFI.MessageRole.ASSISTANT, "hey! can i do something for you?"),
            client.createMessage(GooseFFI.MessageRole.USER, prompt)
        )

        try {
            val response = client.performCompletion(
                provider = "databricks",
                modelName = "goose-claude-3-5-sonnet",
                systemPreamble = "You are a helpful assistant",
                messages = messages,
                tools = tools,
                checkToolApproval = true
            )

            println("\nResponse:")
            println(response)
        } catch (e: Exception) {
            println("Error: ${e.message}")
            e.printStackTrace()
        }
    }
}