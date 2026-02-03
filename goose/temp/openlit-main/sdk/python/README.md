<div align="center">
<img src="https://github.com/openlit/.github/blob/main/profile/assets/wide-logo-no-bg.png?raw=true" alt="OpenLIT Logo" width="30%">
<h3>OpenTelemetry-native</h3>
<h1>AI Observability, Evaluation and Guardrails Framework</h1>

**[Documentation](https://docs.openlit.io/) | [Quickstart](#-getting-started-with-llm-observability) | [Roadmap](#️-roadmap) | [Feature Request](https://github.com/openlit/openlit/issues/new?assignees=&labels=%3Araised_hand%3A+Up+for+Grabs%2C+%3Arocket%3A+Feature&projects=&template=feature-request.md&title=%5BFeat%5D%3A) | [Report a Bug](https://github.com/openlit/openlit/issues/new?assignees=&labels=%3Abug%3A+Bug%2C+%3Araised_hand%3A+Up+for+Grabs&projects=&template=bug.md&title=%5BBug%5D%3A)** 

[![OpenLIT](https://img.shields.io/badge/OpenLIT-orange)](https://github.com/openlit/openlit)
[![License](https://img.shields.io/github/license/openlit/openlit?label=License&logo=github&color=f80&logoColor=white)](https://github.com/openlit/openlit/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/openlit/month)](https://pepy.tech/project/openlit)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/openlit/openlit)](https://github.com/openlit/openlit/pulse)
[![GitHub Contributors](https://img.shields.io/github/contributors/openlit/openlit)](https://github.com/openlit/openlit/graphs/contributors)

[![Slack](https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white)](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ)
[![X](https://img.shields.io/badge/follow-%40openlit__io-1DA1F2?logo=x&style=social)](https://twitter.com/openlit_io)

![OpenLIT Connections Banner](https://github.com/openlit/.github/blob/main/profile/assets/openlit-integrations-banner.png?raw=true)


</div>

OpenLIT SDK is a monitoring framework built on top of **OpenTelemetry** that gives your complete Observability for your AI stack, from LLMs to vector databases and GPUs, with just one line of code with tracing and metrics. It also allows you to send the generated traces and metrics to your existing monitoring tools like Grafana, New Relic, and more.

This project proudly follows and maintains the [Semantic Conventions](https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai) with the OpenTelemetry community, consistently updating to align with the latest standards in Observability.

## ⚡ Features

- 🔎 **Auto Instrumentation**: Works with 50+ LLM providers, Agents, Vector databases, and GPUs with just one line of code.
- 🔭 **OpenTelemetry-Native Observability SDKs**: Vendor-neutral SDKs that can send traces and metrics to your existing observability tool like Prometheus and Jaeger.
- 💲 **Cost Tracking for Custom and Fine-Tuned Models**: Pass custom pricing files for accurate budgeting of custom and fine-tuned models.
- 🚀 **Suppport for OpenLIT Features**: Includes suppprt for prompt management and secrets management features available in OpenLIT.

## Auto Instrumentation Capabilities

| LLMs                                                                      | Vector DBs                                   | Frameworks                                      | GPUs          |
|--------------------------------------------------------------------------|----------------------------------------------|-------------------------------------------------|---------------|
| [✅ OpenAI](https://docs.openlit.io/latest/integrations/openai)                      | [✅ ChromaDB](https://docs.openlit.io/latest/integrations/chromadb)       | [✅ Langchain](https://docs.openlit.io/latest/integrations/langchain)      | [✅ NVIDIA](https://docs.openlit.io/latest/integrations/nvidia-gpu) |
| [✅ Ollama](https://docs.openlit.io/latest/integrations/ollama)                      | [✅ Pinecone](https://docs.openlit.io/latest/integrations/pinecone)       | [✅ LiteLLM](https://docs.openlit.io/latest/integrations/litellm)          | [✅ AMD](https://docs.openlit.io/latest/integrations/amd-gpu)      |
| [✅ Anthropic](https://docs.openlit.io/latest/integrations/anthropic)                | [✅ Qdrant](https://docs.openlit.io/latest/integrations/qdrant)           | [✅ LlamaIndex](https://docs.openlit.io/latest/integrations/llama-index)   |               |
| [✅ GPT4All](https://docs.openlit.io/latest/integrations/gpt4all)                    | [✅ Milvus](https://docs.openlit.io/latest/integrations/milvus)           | [✅ Haystack](https://docs.openlit.io/latest/integrations/haystack)        |               |
| [✅ Cohere](https://docs.openlit.io/latest/integrations/cohere)                      | [✅ AstraDB](https://docs.openlit.io/latest/integrations/astradb)         | [✅ EmbedChain](https://docs.openlit.io/latest/integrations/embedchain)    |               |
| [✅ Mistral](https://docs.openlit.io/latest/integrations/mistral)                    |                                                                          | [✅ Guardrails](https://docs.openlit.io/latest/integrations/guardrails)    |               |
| [✅ Azure OpenAI](https://docs.openlit.io/latest/integrations/azure-openai)          |                                                                          | [✅ CrewAI](https://docs.openlit.io/latest/integrations/crewai)            |               |
| [✅ Azure AI Inference](https://docs.openlit.io/latest/integrations/azure-ai-inference) |                                                                          | [✅ DSPy](https://docs.openlit.io/latest/integrations/dspy)                |               |
| [✅ GitHub AI Models](https://docs.openlit.io/latest/integrations/github-models)  |                                                                          | [✅ AG2](https://docs.openlit.io/latest/integrations/ag2)                  |               |
| [✅ HuggingFace Transformers](https://docs.openlit.io/latest/integrations/huggingface) |                                                                          | [✅ Dynamiq](https://docs.openlit.io/latest/integrations/dynamiq)          |               |
| [✅ Amazon Bedrock](https://docs.openlit.io/latest/integrations/bedrock)             |                                                                          | [✅ Phidata](https://docs.openlit.io/latest/integrations/phidata)          |               |
| [✅ AI21](https://docs.openlit.io/latest/integrations/ai21)                          |                                                                          | [✅ mem0](https://docs.openlit.io/latest/integrations/mem0)                |               |
| [✅ Vertex AI](https://docs.openlit.io/latest/integrations/vertexai)                 |                                                                          | [✅ MultiOn](https://docs.openlit.io/latest/integrations/multion)          |               |
| [✅ Groq](https://docs.openlit.io/latest/integrations/groq)                          |                                                                          | [✅ Julep AI](https://docs.openlit.io/latest/integrations/julep-ai)        |               |
| [✅ ElevenLabs](https://docs.openlit.io/latest/integrations/elevenlabs)              |                                                                          | [✅ ControlFlow](https://docs.openlit.io/latest/integrations/controlflow)  |               |
| [✅ vLLM](https://docs.openlit.io/latest/integrations/vllm)                          |                                                                          | [✅ Crawl4AI](https://docs.openlit.io/latest/integrations/crawl4ai)        |               |
| [✅ OLA Krutrim](https://docs.openlit.io/latest/integrations/krutrim)                |                                                                          | [✅ FireCrawl](https://docs.openlit.io/latest/integrations/firecrawl)      |               |
| [✅ Google AI Studio](https://docs.openlit.io/latest/integrations/google-ai-studio)  |                                                                          | [✅ Letta](https://docs.openlit.io/latest/integrations/letta)              |               |
| [✅ NVIDIA NIM](https://docs.openlit.io/latest/integrations/nvidia-nim)              |                                                                          | [✅ SwarmZero](https://docs.openlit.io/latest/integrations/swarmzero)      |               |
| [✅ Titan ML](https://docs.openlit.io/latest/integrations/titan-ml)                  |                                                                          |                                                |               |
| [✅ Reka AI](https://docs.openlit.io/latest/integrations/reka)                       |                                                                          |                                                |               |
| [✅ xAI](https://docs.openlit.io/latest/integrations/xai)                            |                                                                          |                                                |               |
| [✅ Prem AI](https://docs.openlit.io/latest/integrations/premai)                     |                                                                          |                                                |               |
| [✅ Assembly AI](https://docs.openlit.io/latest/integrations/assemblyai)             |                                                                          |                                                |               |
| [✅ Together](https://docs.openlit.io/latest/integrations/together)                  |                                                                          |                                                |               |
| [✅ DeepSeek](https://docs.openlit.io/latest/integrations/deepseek)                                                                     |                                                                          |                                                |               |

## Supported Destinations
- [✅ OpenTelemetry Collector](https://docs.openlit.io/latest/connections/otelcol)
- [✅ Prometheus + Tempo](https://docs.openlit.io/latest/connections/prometheus-tempo)
- [✅ Prometheus + Jaeger](https://docs.openlit.io/latest/connections/prometheus-jaeger)
- [✅ Grafana Cloud](https://docs.openlit.io/latest/connections/grafanacloud)
- [✅ New Relic](https://docs.openlit.io/latest/connections/new-relic)
- [✅ Elastic](https://docs.openlit.io/latest/connections/elastic)
- [✅ Middleware.io](https://docs.openlit.io/latest/connections/middleware)
- [✅ HyperDX](https://docs.openlit.io/latest/connections/hyperdx)
- [✅ DataDog](https://docs.openlit.io/latest/connections/datadog)
- [✅ SigNoz](https://docs.openlit.io/latest/connections/signoz)
- [✅ OneUptime](https://docs.openlit.io/latest/connections/oneuptime)
- [✅ Dynatrace](https://docs.openlit.io/latest/connections/dynatrace)
- [✅ OpenObserve](https://docs.openlit.io/latest/connections/openobserve)
- [✅ Highlight.io](https://docs.openlit.io/latest/connections/highlight)
- [✅ SigLens](https://docs.openlit.io/latest/connections/siglens)
- [✅ Oodle](https://docs.openlit.io/latest/connections/oodle)

## 💿 Installation

```bash
pip install openlit
```

## 🚀 Getting Started with LLM Observability

### Step 1: Install OpenLIT SDK

Open your command line or terminal and run:

```bash
pip install openlit
```

### Step 2: Initialize OpenLIT in your Application
Integrate OpenLIT into your AI applications by adding the following lines to your code.

```python
import openlit

openlit.init()
```

Configure the telemetry data destination as follows:

| Purpose                                   | Parameter/Environment Variable                   | For Sending to OpenLIT         |
|-------------------------------------------|--------------------------------------------------|--------------------------------|
| Send data to an HTTP OTLP endpoint        | `otlp_endpoint` or `OTEL_EXPORTER_OTLP_ENDPOINT` | `"http://127.0.0.1:4318"`     |
| Authenticate telemetry backends           | `otlp_headers` or `OTEL_EXPORTER_OTLP_HEADERS`   | Not required by default       |

> 💡 Info: If the `otlp_endpoint` or `OTEL_EXPORTER_OTLP_ENDPOINT` is not provided, the OpenLIT SDK will output traces directly to your console, which is recommended during the development phase.

#### Example

---

<details>
  <summary>Initialize using Function Arguments</summary>
  
  ---

  Add the following two lines to your application code:
  
  ```python
  import openlit
  
  openlit.init(
    otlp_endpoint="YOUR_OTEL_ENDPOINT", 
    otlp_headers ="YOUR_OTEL_ENDPOINT_AUTH"
  )
  ```

</details>

---

<details>

  <summary>Initialize using Environment Variables</summary>

  ---

  Add the following two lines to your application code:

  ```python
  import openlit

  openlit.init()
  ```
  
  Then, configure the your OTLP endpoint using environment variable:

  ```env
  export OTEL_EXPORTER_OTLP_ENDPOINT = "YOUR_OTEL_ENDPOINT"
  export OTEL_EXPORTER_OTLP_HEADERS = "YOUR_OTEL_ENDPOINT_AUTH"
  ```
</details>

---

### Step 3: Visualize and Optimize!
Now that your LLM observability data is being collected and sent to configured OpenTelemetry destination, the next step is to visualize and analyze this data. This will help you understand your LLM application's performance and behavior and identify where it can be improved.

If you want to use OpenLIT's Observability Dashboard to monitor LLM usage—like cost, tokens, and user interactions—please check out our [Quickstart Guide](https://docs.openlit.io/latest/quickstart).

If you're sending metrics and traces to other observability tools, take a look at our [Connections Guide](https://docs.openlit.io/latest/destinations/intro) to start using a pre-built dashboard we have created for these tools.

![](https://github.com/openlit/.github/blob/main/profile/assets/openlit-client-1.png?raw=true)


## Configuration

### Observability - `openlit.init()`

Below is a detailed overview of the configuration options available, allowing you to adjust OpenLIT's behavior and functionality to align with your specific observability needs:


| Argument                | Description                                                                                   | Default Value  | Required |
|-------------------------|-----------------------------------------------------------------------------------------------|----------------|----------|
| `environment`           | The deployment environment of the application.                                                | `"default"`    |    Yes   |
| `application_name`      | Identifies the name of your application.                                                      | `"default"`    |    Yes   |
| `tracer`                | An instance of OpenTelemetry Tracer for tracing operations.                                   | `None`         |    No    |
| `meter`                 | An OpenTelemetry Metrics instance for capturing metrics.                                      | `None`         |    No    |
| `otlp_endpoint`         | Specifies the OTLP endpoint for transmitting telemetry data.                                  | `None`         |    No    |
| `otlp_headers`          | Defines headers for the OTLP exporter, useful for backends requiring authentication.          | `None`         |    No    |
| `disable_batch`         | A flag to disable batch span processing, favoring immediate dispatch.                         | `False`        |    No    |
| `capture_message_content`         | Enables tracing of content for deeper insights.                                               | `True`         |    No    |
| `disabled_instrumentors`| List of instrumentors to disable. | `None` |    No    |
| `disable_metrics`       | If set, disables the collection of metrics.                                                   | `False`        |    No    |
| `pricing_json`          | URL or file path of the pricing JSON file.                                             | `https://github.com/openlit/openlit/blob/main/assets/pricing.json`        |    No    |
| `collect_gpu_stats`          | Flag to enable or disable GPU metrics collection.                                         | `False`        |    No    |

### OpenLIT Prompt Hub - `openlit.get_prompt()`

Below are the parameters for use with the SDK for OpenLIT Prompt Hub for prompt management:

| Parameter         | Description                                                                                                                        |
|-------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `url`             | Sets the OpenLIT URL. Defaults to the `OPENLIT_URL` environment variable.                                                          |
| `api_key`         | Sets the OpenLIT API Key. Can also be provided via the `OPENLIT_API_KEY` environment variable.                                     |
| `name`            | Sets the name to fetch a unique prompt. Use this or `prompt_id`.                                                                    |
| `prompt_id`       | Sets the ID to fetch a unique prompt. Use this or `name`. Optional                                                                 |
| `version`         | Set to `True` to get the prompt with variable substitution.. Optional                                                              |
| `shouldCompile`        | Boolean value that compiles the prompt using the provided variables. Optional                                                      |
| `variables`       | Sets the variables for prompt compilation. Optional                                                                                |
| `meta_properties` | Sets the meta-properties for storing in the prompt's access history metadata. Optional                                             |

### OpenLIT Vault - `openlit.get_secrets()`

Below are the parameters for use with the SDK for OpenLIT Vault for secret management:

| Parameter         | Description                                                                                                                        |
|-------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `url`             | Sets the Openlit URL. Defaults to the `OPENLIT_URL` environment variable.                                                          |
| `api_key`         | Sets the OpenLIT API Key. Can also be provided via the `OPENLIT_API_KEY` environment variable.                                     |
| `key`            | Sets the key to fetch a specific secret.     Optional                                                   |
| `should_set_env`        | Boolean value that sets all the secrets as environment variables for the application. Optional                                                     |
| `tags`       | Sets the tags for fetching only the secrets that have the mentioned tags assigned. Optional                                                                                |


## 🛣️ Roadmap

We are dedicated to continuously improving OpenLIT SDKs. Here's a look at what's been accomplished and what's on the horizon:

| Feature                                                                                      | Status        |
|----------------------------------------------------------------------------------------------|---------------|
| [OpenTelmetry auto-instrumentation for LLM Providers like OpenAI, Anthropic]()               | ✅ Completed  |
| [OpenTelmetry auto-instrumentation for Vector databases like Pinecone, Chroma]()             | ✅ Completed  |
| [OpenTelmetry auto-instrumentation for LLM Frameworks like LangChain, LlamaIndex]()          | ✅ Completed  |
| [OpenTelemetry-native auto-instrumentation for NVIDIA GPU Monitoring](https://docs.openlit.io/latest/features/gpu)           | ✅ Completed  |
| [Real-Time Guardrails Implementation](https://docs.openlit.io/latest/features/guardrails)                                                      | ✅ Completed    |
| [Programmatic Evaluation for LLM Response](https://docs.openlit.io/latest/features/evaluations)                                                      | ✅ Completed    |
| [OpenTelemetry-native AI Agent Observability]()             | ✅ Completed  |


## 🌱 Contributing

Whether it's big or small, we love contributions 💚. Check out our [Contribution guide](../../CONTRIBUTING.md) to get started

Unsure where to start? Here are a few ways to get involved:

- Join our [Slack](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ) or [Discord](https://discord.gg/rjvTm6zd) community to discuss ideas, share feedback, and connect with both our team and the wider OpenLIT community.

Your input helps us grow and improve, and we're here to support you every step of the way.

## 💚 Community & Support

Connect with the OpenLIT community and maintainers for support, discussions, and updates:

- 🌟 If you like it, Leave a star on our [GitHub](https://github.com/openlit/openlit/)
- 🌍 Join our [Slack](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ) or [Discord](https://discord.gg/CQnXwNT3) community for live interactions and questions.
- 🐞 Report bugs on our [GitHub Issues](https://github.com/openlit/openlit/issues) to help us improve OpenLIT.
- 𝕏 Follow us on [X](https://x.com/openlit_io) for the latest updates and news.
