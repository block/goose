import io.github.aaif_goose.MessageRole
import io.github.aaif_goose.ProviderMessage
import io.github.aaif_goose.ProviderModelConfig
import io.github.aaif_goose.streamFlow
import io.github.aaif_goose.providers.openai.defaultModel
import io.github.aaif_goose.providers.openai.provider as openAiProvider
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val apiKey = System.getenv("OPENAI_API_KEY")
    require(!apiKey.isNullOrBlank()) {
        "Set OPENAI_API_KEY before running this example."
    }

    val provider = openAiProvider(apiKey)
    val model = ProviderModelConfig(modelName = defaultModel())
    val messages = listOf(
        ProviderMessage(
            role = MessageRole.USER,
            text = "What is the capital of France? Answer in one sentence.",
        ),
    )

    provider
        .streamFlow(
            model,
            "You are a knowledgeable geography expert.",
            messages,
        )
        .collect { chunk ->
            chunk.text?.let { print(it) }
            chunk.usageJson?.let { println("\nusage: $it") }
        }
    println()
}
