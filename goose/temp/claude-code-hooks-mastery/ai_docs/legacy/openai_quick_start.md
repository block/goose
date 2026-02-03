# Developer quickstart

Take your first steps with the OpenAI API.

The OpenAI API provides a simple interface to state-of-the-art AI [models](https://platform.openai.com/docs/models) for text generation, natural language processing, computer vision, and more. This example generates [text output](https://platform.openai.com/docs/guides/text) from a prompt, as you might using [ChatGPT](https://chatgpt.com/).

## Generate text from a model

### JavaScript

```javascript
import OpenAI from "openai";
const client = new OpenAI();

const response = await client.responses.create({
    model: "gpt-4.1",
    input: "Write a one-sentence bedtime story about a unicorn."
});

console.log(response.output_text);
```

### Python

```python
from openai import OpenAI
client = OpenAI()

response = client.responses.create(
    model="gpt-4.1",
    input="Write a one-sentence bedtime story about a unicorn."
)

print(response.output_text)
```

### cURL

```bash
curl "https://api.openai.com/v1/responses" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -d '{
        "model": "gpt-4.1",
        "input": "Write a one-sentence bedtime story about a unicorn."
    }'
```

### Data retention for model responses

Response objects are saved for 30 days by default. They can be viewed in the dashboard
[logs](https://platform.openai.com/logs?api=responses) page or
[retrieved](https://platform.openai.com/docs/api-reference/responses/get) via the API.
You can disable this behavior by setting `store` to `false`
when creating a Response.

OpenAI does not use data sent via API to train our models without your explicit consent— [learn more](https://platform.openai.com/docs/guides/your-data).

## Analyze image inputs

You can provide image inputs to the model as well. Scan receipts, analyze screenshots, or find objects in the real world with [computer vision](https://platform.openai.com/docs/guides/images).

### Analyze the content of an image

#### JavaScript

```javascript
import OpenAI from "openai";
const client = new OpenAI();

const response = await client.responses.create({
    model: "gpt-4.1",
    input: [
        { role: "user", content: "What two teams are playing in this photo?" },
        {
            role: "user",
            content: [
                {
                    type: "input_image",
                    image_url: "https://upload.wikimedia.org/wikipedia/commons/3/3b/LeBron_James_Layup_%28Cleveland_vs_Brooklyn_2018%29.jpg",
                }
            ],
        },
    ],
});

console.log(response.output_text);
```

#### Python

```python
from openai import OpenAI
client = OpenAI()

response = client.responses.create(
    model="gpt-4.1",
    input=[
        {"role": "user", "content": "what teams are playing in this image?"},
        {
            "role": "user",
            "content": [
                {
                    "type": "input_image",
                    "image_url": "https://upload.wikimedia.org/wikipedia/commons/3/3b/LeBron_James_Layup_%28Cleveland_vs_Brooklyn_2018%29.jpg"
                }
            ]
        }
    ]
)

print(response.output_text)
```

#### cURL

```bash
curl "https://api.openai.com/v1/responses" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -d '{
        "model": "gpt-4.1",
        "input": [
            {
                "role": "user",
                "content": "What two teams are playing in this photo?"
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_image",
                        "image_url": "https://upload.wikimedia.org/wikipedia/commons/3/3b/LeBron_James_Layup_%28Cleveland_vs_Brooklyn_2018%29.jpg"
                    }
                ]
            }
        ]
    }'
```

## Extend the model with tools

Give the model access to new data and capabilities using [tools](https://platform.openai.com/docs/guides/tools). You can either call your own [custom code](https://platform.openai.com/docs/guides/function-calling), or use one of OpenAI's [powerful built-in tools](https://platform.openai.com/docs/guides/tools). This example uses [web search](https://platform.openai.com/docs/guides/tools-web-search) to give the model access to the latest information on the Internet.

### Get information for the response from the Internet

#### JavaScript

```javascript
import OpenAI from "openai";
const client = new OpenAI();

const response = await client.responses.create({
    model: "gpt-4.1",
    tools: [ { type: "web_search_preview" } ],
    input: "What was a positive news story from today?",
});

console.log(response.output_text);
```

#### Python

```python
from openai import OpenAI
client = OpenAI()

response = client.responses.create(
    model="gpt-4.1",
    tools=[{"type": "web_search_preview"}],
    input="What was a positive news story from today?"
)

print(response.output_text)
```

#### cURL

```bash
curl "https://api.openai.com/v1/responses" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -d '{
        "model": "gpt-4.1",
        "tools": [{"type": "web_search_preview"}],
        "input": "what was a positive news story from today?"
    }'
```

## Deliver blazing fast AI experiences

Using either the new [Realtime API](https://platform.openai.com/docs/guides/realtime) or server-sent [streaming events](https://platform.openai.com/docs/guides/streaming-responses), you can build high performance, low-latency experiences for your users.

### Stream server-sent events from the API

#### JavaScript

```javascript
import { OpenAI } from "openai";
const client = new OpenAI();

const stream = await client.responses.create({
    model: "gpt-4.1",
    input: [
        {
            role: "user",
            content: "Say 'double bubble bath' ten times fast.",
        },
    ],
    stream: true,
});

for await (const event of stream) {
    console.log(event);
}
```

#### Python

```python
from openai import OpenAI
client = OpenAI()

stream = client.responses.create(
    model="gpt-4.1",
    input=[
        {
            "role": "user",
            "content": "Say 'double bubble bath' ten times fast.",
        },
    ],
    stream=True,
)

for event in stream:
    print(event)
```

## Build agents

Use the OpenAI platform to build [agents](https://platform.openai.com/docs/guides/agents) capable of taking action—like [controlling computers](https://platform.openai.com/docs/guides/tools-computer-use)—on behalf of your users. Use the Agents SDK for [Python](https://openai.github.io/openai-agents-python) or [TypeScript](https://openai.github.io/openai-agents-js) to create orchestration logic on the backend.

### Build a language triage agent

#### JavaScript

```javascript
import { Agent, run } from '@openai/agents';

const spanishAgent = new Agent({
  name: 'Spanish agent',
  instructions: 'You only speak Spanish.',
});

const englishAgent = new Agent({
  name: 'English agent',
  instructions: 'You only speak English',
});

const triageAgent = new Agent({
  name: 'Triage agent',
  instructions:
    'Handoff to the appropriate agent based on the language of the request.',
  handoffs: [spanishAgent, englishAgent],
});

const result = await run(triageAgent, 'Hola, ¿cómo estás?');
console.log(result.finalOutput);
```

#### Python

```python
from agents import Agent, Runner
import asyncio

spanish_agent = Agent(
    name="Spanish agent",
    instructions="You only speak Spanish.",
)

english_agent = Agent(
    name="English agent",
    instructions="You only speak English",
)

triage_agent = Agent(
    name="Triage agent",
    instructions="Handoff to the appropriate agent based on the language of the request.",
    handoffs=[spanish_agent, english_agent],
)

async def main():
    result = await Runner.run(triage_agent, input="Hola, ¿cómo estás?")
    print(result.final_output)

if __name__ == "__main__":
    asyncio.run(main())
```

## Explore further

We've barely scratched the surface of what's possible with the OpenAI platform. Here are some resources you might want to explore next.

- **Go deeper with prompting and text generation** - Learn more about prompting, message roles, and building conversational apps like chat bots.
- **Analyze the content of images** - Learn to use image inputs to the model and extract meaning from images.
- **Generate structured JSON data from the model** - Generate JSON data from the model that conforms to a JSON schema you specify.
- **Call custom code to help generate a response** - Empower the model to invoke your own custom code to help generate a response. Do this to give the model access to data or systems it wouldn't be able to access otherwise.
- **Search the web or use your own data in responses** - Try out powerful built-in tools to extend the capabilities of the models. Search the web or your own data for up-to-date information the model can use to generate responses.
- **Responses starter app** - Start building with the Responses API
- **Build agents** - Explore interfaces to build powerful AI agents that can take action on behalf of users. Control a computer to take action on behalf of a user, or orchestrate multi-agent flows with the Agents SDK.
- **Full API Reference** - View the full API reference for the OpenAI platform.