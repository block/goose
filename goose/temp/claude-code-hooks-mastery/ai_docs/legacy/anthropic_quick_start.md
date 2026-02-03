# Get started with Claude

In this example, we'll have Claude write a Python function that checks if a string is a palindrome.

## Prerequisites

You will need:

- An Anthropic [Console account](https://console.anthropic.com/)
- An [API key](https://console.anthropic.com/settings/keys)
- Python 3.7+ or TypeScript 4.5+

Anthropic provides [Python and TypeScript SDKs](https://docs.anthropic.com/en/api/client-sdks), although you can make direct HTTP requests to the API.

## Start with the Workbench

Any API call you make—regardless of the specific task—sends a well-configured prompt to the Anthropic API. As you're learning to make the most of Claude, we recommend that you start the development process in the Workbench, a web-based interface to Claude.

Log into the [Anthropic Console](https://console.anthropic.com/) and click **Write a prompt from scratch**.

In the middle section, under User, let's ask Claude a question.

User

```text
Why is the ocean salty?
```

Click **Run**. On the right side, you'll see output like

Response

```text
The ocean is salty due to several factors:

1. Weathering of rocks: Over millions of years, rain, rivers, and streams have eroded rocks containing mineral salts. These salts are carried into the ocean by water runoff.
2. Volcanic activity: Underwater volcanoes and hydrothermal vents release minerals, including salts, into the ocean water.
3. Atmospheric deposition: Salt particles from ocean spray can be carried by wind and deposited back into the ocean.
4. Evaporation: As water evaporates from the surface of the ocean, it leaves behind dissolved salts, increasing the concentration of salt in the remaining water.
5. Biological processes: Some marine organisms contribute to the ocean's salinity by releasing salt compounds as byproducts of their metabolism.

Over time, these processes have continuously added salts to the ocean, while evaporation removes pure water, leading to the ocean's current salinity levels. It's important to note that the total amount of salt in the ocean remains relatively stable because the input of salts is balanced by the removal of salts through processes like the formation of evaporite deposits.
```

This is a good answer, but let's say we wanted to control the exact type of answer Claude gives. For example, only allowing Claude to respond to questions with poems. We can control the format, tone, and personality of the response by adding a System Prompt.

System prompt

```text
You are a world-class poet. Respond only with short poems.
```

Click **Run** again.

Response

```text
The ocean's salty brine,
A tale of time and elements combined.
Rocks and rain, a slow erosion,
Minerals carried in solution.
Eons pass, the salt remains,
In the vast, eternal watery domain.
```

See how Claude's response has changed? LLMs respond well to clear and direct instructions. You can put the role instructions in either the system prompt or the user message. We recommend testing to see which way yields the best results for your use case.

Once you've tweaked the inputs such that you're pleased with the output and have a good sense how to use Claude, convert your Workbench into an integration.

Click **Get Code** to copy the generated code representing your Workbench session.

## Install the SDK

Anthropic provides SDKs for [Python](https://pypi.org/project/anthropic/) (3.7+), [TypeScript](https://www.npmjs.com/package/@anthropic-ai/sdk) (4.5+), and [Java](https://central.sonatype.com/artifact/com.anthropic/anthropic-java/) (8+). We also currently have a [Go](https://pkg.go.dev/github.com/anthropics/anthropic-sdk-go) SDK in beta.

### Python

In your project directory, create a virtual environment.

```bash
python -m venv claude-env
```

Activate the virtual environment using

- On macOS or Linux, `source claude-env/bin/activate`
- On Windows, `claude-env\Scripts\activate`

```bash
pip install anthropic
```

### TypeScript

Install the SDK.

```bash
npm install @anthropic-ai/sdk
```

### Java

First find the current version of the Java SDK on [Maven Central](https://central.sonatype.com/artifact/com.anthropic/anthropic-java).
Declare the SDK as a dependency in your Gradle file:

```gradle
implementation("com.anthropic:anthropic-java:1.0.0")
```

Or in your Maven file:

```xml
<dependency>
  <groupId>com.anthropic</groupId>
  <artifactId>anthropic-java</artifactId>
  <version>1.0.0</version>
</dependency>
```

## Set your API key

Every API call requires a valid API key. The SDKs are designed to pull the API key from an environmental variable `ANTHROPIC_API_KEY`. You can also supply the key to the Anthropic client when initializing it.

### macOS and Linux

```bash
export ANTHROPIC_API_KEY='your-api-key-here'
```

## Call the API

Call the API by passing the proper parameters to the [/messages](https://docs.anthropic.com/en/api/messages) endpoint.

Note that the code provided by the Workbench sets the API key in the constructor. If you set the API key as an environment variable, you can omit that line as below.

### Python

```python
import anthropic

client = anthropic.Anthropic()

message = client.messages.create(
    model="claude-opus-4-20250514",
    max_tokens=1000,
    temperature=1,
    system="You are a world-class poet. Respond only with short poems.",
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "Why is the ocean salty?"
                }
            ]
        }
    ]
)
print(message.content)
```

Run the code using `python3 claude_quickstart.py` or `node claude_quickstart.js`.

Output (Python)

```python
[TextBlock(text="The ocean's salty brine,\nA tale of time and design.\nRocks and rivers, their minerals shed,\nAccumulating in the ocean's bed.\nEvaporation leaves salt behind,\nIn the vast waters, forever enshrined.", type='text')]
```

The Workbench and code examples use default model settings for: model (name), temperature, and max tokens to sample.

This quickstart shows how to develop a basic, but functional, Claude-powered application using the Console, Workbench, and API. You can use this same workflow as the foundation for much more powerful use cases.

## Next steps

Now that you have made your first Anthropic API request, it's time to explore what else is possible:

- **Use Case Guides** - End to end implementation guides for common use cases.
- **Anthropic Cookbook** - Learn with interactive Jupyter notebooks that demonstrate uploading PDFs, embeddings, and more.
- **Prompt Library** - Explore dozens of example prompts for inspiration across use cases.