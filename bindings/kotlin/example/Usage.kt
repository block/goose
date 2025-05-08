// bindings/kotlin/example/Usage.kt
import uniffi.goose_llm.*

fun main() {
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
        )
    )

    printMessages(msgs)
}
