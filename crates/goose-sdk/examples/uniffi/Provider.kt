package aaif.example

import io.aaif.goose.DeclarativeProvider
import io.aaif.goose.MessageRole
import io.aaif.goose.ProviderMessage
import io.aaif.goose.ProviderModelConfig
import java.nio.file.Paths

fun main() {
    val examplesDir = Paths.get("crates/goose-sdk/examples")
    val provider = DeclarativeProvider.fromJson(examplesDir.resolve("deepseek.json").toFile().readText())
    val model = ProviderModelConfig(modelName = "deepseek-v4-flash")
    val messages = listOf(
        ProviderMessage(
            role = MessageRole.USER,
            text = "what is the capital of France?",
        ),
    )
    val stream = provider.stream(
        model,
        "You are a knowledgable geography expert",
        messages,
    )

    while (true) {
        val chunk = stream.next() ?: break
        chunk.text?.let { print(it) }
        chunk.usageJson?.let { println("\nusage: $it") }
    }
    println()
}
