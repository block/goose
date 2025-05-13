package com.example.goose

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure
import com.sun.jna.ptr.PointerByReference
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * Kotlin example for using the Goose FFI interface.
 * 
 * This example demonstrates how to:
 * 1. Load the Goose FFI library using JNA
 * 2. Create an agent with a provider
 * 3. Use the ReplyState system
 * 4. Handle tool approvals
 * 5. Process conversations
 */

// Enums
enum class ReplyProcessState(val value: Int) {
    READY(0),
    WAITING_FOR_PROVIDER(1),
    MESSAGE_YIELDED(2),
    WAITING_FOR_TOOL_APPROVAL(3),
    PROCESSING_TOOLS(4),
    COMPLETED(5),
    ERROR(6);
    
    companion object {
        fun fromValue(value: Int): ReplyProcessState {
            return values().find { it.value == value } ?: ERROR
        }
    }
}

enum class MessageRole(val value: Int) {
    USER(0),
    ASSISTANT(1),
    SYSTEM(2)
}

// JNA Structures
class ProviderConfig : Structure() {
    @JvmField var provider_type: Int = 0
    @JvmField var api_key: String? = null
    @JvmField var model_name: String? = null
    @JvmField var host: String? = null
    
    override fun getFieldOrder(): List<String> {
        return listOf("provider_type", "api_key", "model_name", "host")
    }
}

class AsyncResult : Structure() {
    @JvmField var succeeded: Boolean = false
    @JvmField var error_message: String? = null
    
    override fun getFieldOrder(): List<String> {
        return listOf("succeeded", "error_message")
    }
}

class MessageFFI : Structure() {
    @JvmField var role: Int = 0
    @JvmField var content: String? = null
    
    override fun getFieldOrder(): List<String> {
        return listOf("role", "content")
    }
}

class PendingToolRequestFFI : Structure() {
    @JvmField var id: String? = null
    @JvmField var name: String? = null
    @JvmField var arguments: String? = null
    @JvmField var requires_approval: Boolean = false
    
    override fun getFieldOrder(): List<String> {
        return listOf("id", "name", "arguments", "requires_approval")
    }
}

// JNA Library Interface
interface GooseFFI : Library {
    companion object {
        val INSTANCE: GooseFFI = Native.load("goose_ffi", GooseFFI::class.java)
    }
    
    // Agent functions
    fun goose_agent_new(config: ProviderConfig): Pointer?
    fun goose_agent_free(agent: Pointer)
    
    // FFI Agent functions
    fun goose_ffi_agent_new(agent: Pointer): Pointer?
    fun goose_ffi_agent_create_reply_state(
        ffiAgent: Pointer,
        messages: Array<MessageFFI>,
        messageCount: Int,
        sessionConfig: Pointer?
    ): Pointer?
    
    // ReplyState functions
    fun goose_reply_state_start(replyState: Pointer): Pointer?
    fun goose_reply_state_advance(replyState: Pointer): Pointer?
    fun goose_reply_state_get_state(replyState: Pointer): Int
    fun goose_reply_state_get_current_message(replyState: Pointer): String?
    fun goose_reply_state_get_pending_tool_requests(
        replyState: Pointer,
        outLen: PointerByReference
    ): Pointer?
    fun goose_reply_state_approve_tool(replyState: Pointer, requestId: String): Pointer?
    fun goose_reply_state_deny_tool(replyState: Pointer, requestId: String): Pointer?
    fun goose_reply_state_free(replyState: Pointer)
    
    // Cleanup functions
    fun goose_free_string(str: Pointer)
    fun goose_free_async_result(result: Pointer)
}

// Kotlin wrapper classes
class GooseAgent(
    apiKey: String? = null,
    modelName: String? = null,
    host: String? = null
) {
    private val agent: Pointer
    private val ffiAgent: Pointer
    
    init {
        val config = ProviderConfig().apply {
            provider_type = 0 // DATABRICKS
            this.api_key = apiKey
            this.model_name = modelName
            this.host = host
        }
        
        agent = GooseFFI.INSTANCE.goose_agent_new(config)
            ?: throw RuntimeException("Failed to create Goose agent")
        
        ffiAgent = GooseFFI.INSTANCE.goose_ffi_agent_new(agent)
            ?: throw RuntimeException("Failed to create FFI agent")
    }
    
    fun createReplyState(messages: List<Message>): ReplyState {
        val ffiMessages = messages.map { msg ->
            MessageFFI().apply {
                role = msg.role.value
                content = Json.encodeToString(Message.serializer(), msg)
            }
        }.toTypedArray()
        
        val replyStatePtr = GooseFFI.INSTANCE.goose_ffi_agent_create_reply_state(
            ffiAgent,
            ffiMessages,
            ffiMessages.size,
            null
        ) ?: throw RuntimeException("Failed to create reply state")
        
        return ReplyState(replyStatePtr)
    }
    
    fun close() {
        GooseFFI.INSTANCE.goose_agent_free(agent)
    }
}

class ReplyState(private val ptr: Pointer) {
    fun start() {
        val result = GooseFFI.INSTANCE.goose_reply_state_start(ptr)
        checkResult(result)
    }
    
    fun advance() {
        val result = GooseFFI.INSTANCE.goose_reply_state_advance(ptr)
        checkResult(result)
    }
    
    fun getState(): ReplyProcessState {
        val stateValue = GooseFFI.INSTANCE.goose_reply_state_get_state(ptr)
        return ReplyProcessState.fromValue(stateValue)
    }
    
    fun getCurrentMessage(): Message? {
        val messageJson = GooseFFI.INSTANCE.goose_reply_state_get_current_message(ptr)
        return messageJson?.let { Json.decodeFromString(Message.serializer(), it) }
    }
    
    fun getPendingToolRequests(): List<PendingToolRequest> {
        val lengthPtr = PointerByReference()
        val requestsPtr = GooseFFI.INSTANCE.goose_reply_state_get_pending_tool_requests(ptr, lengthPtr)
            ?: return emptyList()
        
        val length = lengthPtr.value.getInt(0)
        val requests = mutableListOf<PendingToolRequest>()
        
        for (i in 0 until length) {
            val requestPtr = requestsPtr.share(i.toLong() * PendingToolRequestFFI().size())
            val request = PendingToolRequestFFI(requestPtr)
            requests.add(PendingToolRequest(
                id = request.id ?: "",
                name = request.name ?: "",
                arguments = request.arguments?.let { Json.parseToJsonElement(it) } ?: Json.parseToJsonElement("{}"),
                requiresApproval = request.requires_approval
            ))
        }
        
        return requests
    }
    
    fun approveTool(requestId: String) {
        val result = GooseFFI.INSTANCE.goose_reply_state_approve_tool(ptr, requestId)
        checkResult(result)
    }
    
    fun denyTool(requestId: String) {
        val result = GooseFFI.INSTANCE.goose_reply_state_deny_tool(ptr, requestId)
        checkResult(result)
    }
    
    fun close() {
        GooseFFI.INSTANCE.goose_reply_state_free(ptr)
    }
    
    private fun checkResult(result: Pointer?) {
        if (result == null) {
            throw RuntimeException("Null result returned")
        }
        
        val asyncResult = AsyncResult(result)
        if (!asyncResult.succeeded) {
            val errorMsg = asyncResult.error_message
            GooseFFI.INSTANCE.goose_free_async_result(result)
            throw RuntimeException("Operation failed: $errorMsg")
        }
        
        GooseFFI.INSTANCE.goose_free_async_result(result)
    }
}

// Data classes
@Serializable
data class Message(
    val role: MessageRole,
    val content: List<MessageContent>
)

@Serializable
data class MessageContent(
    val type: String,
    val text: String? = null
)

data class PendingToolRequest(
    val id: String,
    val name: String,
    val arguments: kotlinx.serialization.json.JsonElement,
    val requiresApproval: Boolean
)

// Example usage
fun main() {
    val apiKey = System.getenv("DATABRICKS_API_KEY")
    val host = System.getenv("DATABRICKS_HOST")
    
    val agent = GooseAgent(
        apiKey = apiKey,
        modelName = "claude-3-7-sonnet",
        host = host
    )
    
    try {
        // Create a message
        val message = Message(
            role = MessageRole.USER,
            content = listOf(MessageContent(type = "text", text = "What is 42 + 58?"))
        )
        
        // Create reply state
        val replyState = agent.createReplyState(listOf(message))
        
        try {
            // Start the conversation
            replyState.start()
            
            // Process the conversation
            while (replyState.getState() != ReplyProcessState.COMPLETED) {
                when (val state = replyState.getState()) {
                    ReplyProcessState.MESSAGE_YIELDED -> {
                        val currentMessage = replyState.getCurrentMessage()
                        currentMessage?.content?.forEach { content ->
                            if (content.type == "text") {
                                println("Agent: ${content.text}")
                            }
                        }
                        replyState.advance()
                    }
                    
                    ReplyProcessState.WAITING_FOR_TOOL_APPROVAL -> {
                        val toolRequests = replyState.getPendingToolRequests()
                        toolRequests.forEach { request ->
                            println("Tool request: ${request.name} with args ${request.arguments}")
                            
                            // For demo, automatically approve calculator
                            if (request.name == "calculator") {
                                replyState.approveTool(request.id)
                            } else {
                                // Ask for approval
                                print("Approve tool ${request.name}? (y/n): ")
                                val approve = readLine()
                                if (approve?.lowercase() == "y") {
                                    replyState.approveTool(request.id)
                                } else {
                                    replyState.denyTool(request.id)
                                }
                            }
                        }
                    }
                    
                    ReplyProcessState.ERROR -> {
                        println("Error occurred in conversation")
                        break
                    }
                    
                    else -> {
                        replyState.advance()
                    }
                }
            }
        } finally {
            replyState.close()
        }
    } finally {
        agent.close()
    }
}