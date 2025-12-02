### Responses API cheat‑sheet

- **Endpoint**: `POST /v1/responses`
- **Basic request**:

```json
{ "model": "gpt-5", "input": "Hello" }
```

- **Streaming**: add `"stream": true` and handle SSE events. See streaming guide.
- **Parameter mapping**:
  - **prompt → input** (string or array of items)
  - **max_tokens → max_output_tokens**
  - **response_format (chat) → text.format (responses)**.
    - Prefer JSON Schema for structured outputs. Required shape (top-level name/schema):
      - `{ text: { format: { type: "json_schema", name: "Output", strict: true, schema: { ... } } } }`
    - If using simple JSON, some SDKs may accept a helper (e.g., `zodResponseFormat`) that produces the correct schema wrapper. Avoid plain strings (e.g., `format: "json"`) which may 400.
  - **temperature**: unchanged
  - **stop, frequency_penalty, presence_penalty**: supported as relevant
  - **tools/function-calling**: use `tools` in Responses
  - **store**: set to `false` by default to avoid retaining response objects and reduce overhead

- **SDK idioms**:
  - Node:

```js
import OpenAI from "openai";
const client = new OpenAI();
const res = await client.responses.create({ model: "gpt-5", input: "Hello" });
console.log(res.output_text);
```

  - Python:

```python
from openai import OpenAI
client = OpenAI()
resp = client.responses.create(model="gpt-5", input="Hello")
print(resp.output_text)
```

- **Streaming (Node)**:

```js
const stream = await client.responses.create({ model: "gpt-5", input: "stream me", stream: true });
for await (const event of stream) {
  if (event.delta) process.stdout.write(event.delta);
}
```

- **Image input (Node)**:

```js
const res = await client.responses.create({
  model: "gpt-5",
  input: [
    { role: "user", content: "What two teams are playing?" },
    { role: "user", content: [ { type: "input_image", image_url: "https://example.com/image.jpg" } ] }
  ]
});
console.log(res.output_text);
```

- **Quick cURL test**:

```bash
curl https://api.openai.com/v1/responses \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-5",
    "input": [{ "role": "user", "content": [{ "type": "input_text", "text": "Return a JSON object with key answer" }] }],
    "text": { "format": { "type": "json_schema", "name": "Output", "strict": true, "schema": { "type": "object", "properties": { "answer": { "type": "string" } }, "required": ["answer"], "additionalProperties": false } } }
  }'
```

- **Tool use**:
  - Define functions in `tools` with JSON Schema params; let the model decide with `tool_choice: "auto"` or force a specific tool.
  - When the model asks to call a tool, execute it in your app and include the tool result in the next request as a `tool` role item within `input`.
  - Keep schemas minimal; validate inputs before execution.
  - Built‑in example (web search preview):

```js
const res = await client.responses.create({
  model: "gpt-5",
  tools: [{ type: "web_search_preview" }],
  input: "What was a positive news story from today?"
});
console.log(res.output_text);
```

- **Reasoning (GPT‑5)**:
  - Migrating from o‑series (`o3`, `o4-mini`, `o1`, `o1-preview`, etc.): `reasoning: { effort: "medium" }`.
  - Migrating from GPT‑series (`gpt-4`, `gpt-4.1`, `gpt-4o`, `gpt-3.5`, etc.): `reasoning: { effort: "minimal" }`.
  - Always include encrypted reasoning tokens; do not emit plaintext chain‑of‑thought. Use the SDK/API option for encrypted reasoning traces where available. Respect any explicit `reasoning.effort` already provided by the app.

  Example (Node):

  ```js
  import OpenAI from "openai";
  const client = new OpenAI();

  const res = await client.responses.create({
    model: "gpt-5",
    input,
    store: false,
    reasoning: { effort: "minimal" }
  });
  console.log(res.output_text);
  ```

  Example (Python):

  ```python
  from openai import OpenAI
  client = OpenAI()

  resp = client.responses.create(
      model="gpt-5",
      input=input_items,
      store=False,
      reasoning={"effort": "minimal"}
  )
  print(resp.output_text)
  ```

- **Multi‑turn**:
  - Manage conversation state in your app; pass prior turns explicitly via `input` items.
  - Use `type: "input_text"` for user/system turns, and `type: "output_text"` for assistant/tool outputs.
  - Always set `store: false`; do not rely on previous message IDs or server‑stored context.

```js
const input = [
  { role: "system", content: [{ type: "input_text", text: "You are helpful." }] },
  { role: "user", content: [{ type: "input_text", text: "Hi" }] },
  { role: "assistant", content: [{ type: "output_text", text: "Hello!" }] },
  { role: "user", content: [{ type: "input_text", text: "Tell me a joke" }] }
];
const res = await client.responses.create({ model: "gpt-5", input });
```

- **Structured output (JSON Schema)**:

```js
const schema = {
  type: "object",
  properties: { answer: { type: "string" } },
  required: ["answer"],
  additionalProperties: false
};
const res = await client.responses.create({
  model: "gpt-5",
  input,
  text: {
    format: { type: "json_schema", json_schema: { name: "Output", strict: true, schema } }
  }
});
const data = JSON.parse(res.output_text);
```

- **Data retention & state**:
  - Set `store: false` on all Responses requests.
  - Do not rely on previous message IDs or server‑stored context; keep state local and minimize logged metadata.

- **Gotchas**:
  - If you previously used Chat Completions for conversation state, manage your own state explicitly with Responses.
  - Prefer `max_output_tokens` over legacy `max_tokens`.
  - When migrating to `gpt-5`, ensure `temperature` is not specified or is set to `1`.
  - Replace Chat `content[].type: "text"` with Responses `content[].type: "input_text"` for user/system inputs.
  - For `text.format`, supply a proper object (e.g., `{ type: "json_schema", name, schema, strict }`), not a plain string.

- **Auto-convert legacy `response_format` (example transformer)**:

```js
function upgradeResponseFormat(body) {
  if (!body || body.text?.format) return body;
  const rf = body.response_format;
  if (!rf) return body;
  let format;
  if (rf === "json") {
    format = { type: "json_schema", name: "Output", strict: true, schema: { type: "object" } };
  } else if (typeof rf === "object") {
    // Attempt to wrap arbitrary schema
    format = { type: "json_schema", name: rf.name || "Output", strict: rf.strict ?? true, schema: rf.schema || { type: "object" } };
  }
  const { response_format, ...rest } = body;
  return format ? { ...rest, text: { format } } : rest;
}
```

- **Troubleshooting 400s**:
  - `missing_required_parameter: text.format.name` → add `json_schema.name` (e.g., "Output").
  - `invalid_type: text.format` → ensure it is an object `{ type, json_schema }`, not a string.
  - `invalid input content type` → use `input_text`/`output_text` content types instead of Chat `text`.


