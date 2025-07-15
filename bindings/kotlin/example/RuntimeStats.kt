import kotlin.system.measureNanoTime
import kotlinx.coroutines.runBlocking
import uniffi.goose_llm.*

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

/* ---------- Goose helpers ---------- */

fun buildProviderConfig(host: String, token: String): String =
    """{ "host": "$host", "token": "$token" }"""

suspend fun timeGooseCall(
    modelCfg: ModelConfig,
    providerName: String,
    providerCfg: String
): Pair<Double, CompletionResponse> {

    val req = createCompletionRequest(
        providerName,
        providerCfg,
        modelCfg,
        systemPreamble = "You are a helpful assistant.",
        messages = listOf(
            Message(
                Role.USER,
                System.currentTimeMillis() / 1000,
                listOf(MessageContent.Text(TextContent("Write me a 1000 word chapter about learning Go vs Rust in the world of LLMs and AI.")))
            )
        ),
        extensions = emptyList()
    )

    lateinit var resp: CompletionResponse
    val wallMs = measureNanoTime { resp = completion(req) } / 1_000_000.0
    return wallMs to resp
}

/* ---------- OpenAI helpers ---------- */

fun timeOpenAiCall(client: HttpClient, apiKey: String): Double {
    val body = """
        {
          "model": "gpt-4.1",
          "max_tokens": 500,
          "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user",   "content": "Write me a 1000 word chapter about learning Go vs Rust in the world of LLMs and AI."}
          ]
        }
    """.trimIndent()

    val request = HttpRequest.newBuilder()
        .uri(URI.create("https://api.openai.com/v1/chat/completions"))
        .header("Authorization", "Bearer $apiKey")
        .header("Content-Type", "application/json")
        .POST(HttpRequest.BodyPublishers.ofString(body))
        .build()

    val wallMs = measureNanoTime {
        client.send(request, HttpResponse.BodyHandlers.ofString())
    } / 1_000_000.0

    return wallMs
}

/* ---------- main ---------- */

fun main() = runBlocking {
    /* Goose provider setup */
    val providerName  = "databricks"
    val host  = System.getenv("DATABRICKS_HOST") ?: error("DATABRICKS_HOST not set")
    val token = System.getenv("DATABRICKS_TOKEN") ?: error("DATABRICKS_TOKEN not set")
    val providerCfg   = buildProviderConfig(host, token)

    /* OpenAI setup */
    val openAiKey = System.getenv("OPENAI_API_KEY") ?: error("OPENAI_API_KEY not set")
    val httpClient = HttpClient.newBuilder().build()

    val gooseModels  = listOf("goose-claude-4-sonnet", "goose-gpt-4-1")
    val runsPerModel = 3

    /* --- Goose timing --- */
    for (model in gooseModels) {
        val maxTokens = 500
        val cfg = ModelConfig(model, 100_000u, 0.0f, maxTokens)
        var wallSum = 0.0
        var gooseSum = 0.0

        println("=== Goose: $model ===")
        repeat(runsPerModel) { run ->
            val (wall, resp) = timeGooseCall(cfg, providerName, providerCfg)
            val gooseMs = resp.runtimeMetrics.totalTimeSec * 1_000
            val overhead = wall - gooseMs
            wallSum += wall
            gooseSum += gooseMs
            println("run ${run + 1}: wall = %.1f ms | goose-llm = %.1f ms | overhead = %.1f ms"
                .format(wall, gooseMs, overhead))
        }
        println("-- avg wall = %.1f ms | avg overhead = %.1f ms --\n"
            .format(wallSum / runsPerModel, (wallSum - gooseSum) / runsPerModel))
    }

    /* --- OpenAI direct timing --- */
    var oaSum = 0.0
    println("=== OpenAI: gpt-4.1 (direct HTTPS) ===")
    repeat(runsPerModel) { run ->
        val wall = timeOpenAiCall(httpClient, openAiKey)
        oaSum += wall
        println("run ${run + 1}: wall = %.1f ms".format(wall))
    }
    println("-- avg wall = %.1f ms --".format(oaSum / runsPerModel))
}
