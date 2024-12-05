# Providers

Providers in Goose mean "LLM providers" that Goose can interact with. Providers are defined in the [Exchange library][exchange-providers] for the most part, but you can define your own.

As you configure your chosen provider, you add the models you want to use to the `~/.config/goose/profiles.yaml` file and you can set any necessary environment variables or API keys in your terminal. For example:
    
```sh
export PROVIDER_API_KEY="your_api_key_here"
```

## Currently Available Providers

### Anthropic

To use Anthropic, you need an API key, which you can obtain by signing up or logging into [Anthropic's platform](https://www.anthropic.com/). Once you have your API key and your `profiles.yaml` file updated to the provider, you can set the `ANTHROPIC_API_KEY` environment variable in your shell using: 

```sh
export ANTHROPIC_API_KEY="your_api_key_here"`.
```

```yaml title="profiles.yaml"
default:
  provider: anthropic
  processor: claude-3-5-sonnet-20241022
  accelerator: claude-3-5-sonnet-20241022
```

### Azure

Azure AI services provide API keys through the Azure Portal. Visit the [Azure Portal](https://portal.azure.com/) to create a resource and obtain your key. You will need to configure Goose by updating your profile and setting appropriate environment variables.

```yaml title="profiles.yaml"
default:
  provider: azure
  processor: azure-gpt-4
  accelerator: azure-gpt-3
```

### Bedrock

More information can be found at [AWS Bedrock](https://aws.amazon.com/bedrock/). You need to set up your AWS credentials and configure Bedrock access accordingly in your Goose profile.


```yaml title="profiles.yaml"
default:
  provider: bedrock
  processor: titan-llm
  accelerator: titan-llm-lite
```

### Databricks

To use Databricks, sign up or log into [Databricks](https://www.databricks.com/) and generate a personal access token via the user settings. Configure Goose by setting the `DATABRICKS_HOST` and `DATABRICKS_TOKEN` environment variables.

```yaml title="profiles.yaml"
default:
  provider: databricks
  processor: databricks-meta-llama-3-1-70b-instruct
  accelerator: databricks-meta-llama-3-1-70b-instruct
```

### Google

Google Cloud AI services require you to set up a project in the [Google Cloud Console](https://console.cloud.google.com/). After enabling the relevant APIs, you should generate an API key or set up a service account. Ensure your application can access these credentials.

```yaml title="profiles.yaml"
default:
  provider: google
  processor: gemini-1.5-flash
  accelerator: gemini-1.5-flash
```

### Ollama

For Ollama, refer to the setup process on [Ollama's site](https://ollama.com/) for obtaining necessary credentials. Make sure your environment has all the required tokens set up.

```yaml title="profiles.yaml"
default:
  provider: ollama
  processor: ollama-pro
  accelerator: ollama-lite
```

### OpenAI

Register at [OpenAI's platform](https://platform.openai.com/api-keys) to obtain an API key. Configure Goose by updating your `profiles.yaml` file and setting the `OPENAI_API_KEY` in your terminal: 

```sh
export OPENAI_API_KEY="your_api_key_here"
```

```yaml title="profiles.yaml"
default:
  provider: openai
  processor: gpt-4
  accelerator: gpt-3.5-turbo
```

[exchange-providers]: https://github.com/block/goose/tree/main/packages/exchange/src/exchange/providers