import io.github.aaif_goose.MessageContent
import io.github.aaif_goose.MessageRole
import io.github.aaif_goose.ProviderMessage
import io.github.aaif_goose.ProviderModelConfig
import io.github.aaif_goose.StreamChunk
import io.github.aaif_goose.cachedSystemText
import io.github.aaif_goose.streamFlow
import io.github.aaif_goose.providers.anthropic.defaultModel
import io.github.aaif_goose.providers.anthropic.provider as anthropicProvider
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val apiKey = System.getenv("ANTHROPIC_API_KEY")
    require(!apiKey.isNullOrBlank()) {
        "Set ANTHROPIC_API_KEY before running this example."
    }

    val provider = anthropicProvider(apiKey)
    val model = ProviderModelConfig(modelName = defaultModel(), reasoning = true)
    val messages = listOf(
        ProviderMessage(
            role = MessageRole.USER,
            content = listOf(
                MessageContent.Text(
                    text = "What is the capital of France? Answer in one sentence.",
                    cacheControl = null,
                ),
            ),
        ),
    )

    provider
        .streamFlow(
            model = model,
            system = listOf(cachedSystemText("You are a knowledgeable geography expert.")),
            messages = messages,
        )
        .collect { chunk ->
            when (chunk) {
                is StreamChunk.TextChunk -> print(chunk.text)
                is StreamChunk.ThinkingChunk -> Unit
                is StreamChunk.RedactedThinkingChunk -> Unit
                is StreamChunk.ToolChunk -> println(
                    "\ntool[${chunk.index}]: ${chunk.name ?: "<pending>"} ${chunk.argumentsJson}",
                )

                is StreamChunk.EndChunk -> chunk.usage?.let { usage ->
                    println(
                        "\nusage: input=${usage.inputTokens}, output=${usage.outputTokens}, " +
                                "cached=${usage.cachedTokens}, accounting=${usage.inputTokenAccounting}",
                    )
                }

                is StreamChunk.ErrorChunk -> System.err.println("\nerror: ${chunk.error.message}")
            }
        }
    println()
}
