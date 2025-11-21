### Migration notes (Completions → Responses)

- **Why migrate**: Responses is the unified API for text, tools, and streaming; Completions/Chat Completions are legacy. With GPT‑5, Responses is required for best performance: GPT‑5 orchestrates tool calls as part of its internal reasoning and relies on Responses’ structured tool flow. Staying on Completions can degrade intelligence and cause repeated tool calls because legacy endpoints don’t preserve the model’s tool context.
- **HTTP**: switch `POST /v1/completions` → `POST /v1/responses`.
- **Fields**: `prompt` → `input`, `max_tokens` → `max_output_tokens`. `temperature` remains.
- **Formatting**: `response_format` from Chat Completions is not supported at the top level. Use `text.format` with a proper object. Prefer JSON Schema: `{ text: { format: { type: "json_schema", name, strict, schema } } }`. Avoid plain strings.
- **Content items**: Replace Chat `content[].type: "text"` with Responses `content[].type: "input_text"` for system/user turns. For assistant or tool outputs, prefer `content[].type: "output_text"`.
- **Images**: Replace Chat image parts with Responses `content[].type: "input_image"` plus `image_url` or image data.

- **GPT‑5 reasoning**:
  - From o‑series (o3, o4‑mini, o1, o1‑preview): set `reasoning: { effort: "medium" }`.
  - From GPT‑series (gpt‑4, gpt‑4.1, gpt‑4o, gpt‑3.5): set `reasoning: { effort: "minimal" }`.
  - Heuristic: inspect the previous `model` string; default to minimal unless it clearly starts with `o`.
  - Always include encrypted reasoning tokens; do not emit plaintext chain‑of‑thought. Use the SDK/API option for encrypted reasoning traces where available.

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

Example transformer (Node/TypeScript):

```ts
function toResponsesPayload(body: any): any {
  const out: any = { ...body };
  // Map response_format → text.format
  if (!out.text?.format && out.response_format) {
    const rf = out.response_format;
    if (rf === "json") {
      out.text = { format: { type: "json_schema", name: "Output", strict: true, schema: { type: "object" } } };
    } else if (typeof rf === "object") {
      out.text = { format: { type: "json_schema", name: rf.name ?? "Output", strict: rf.strict ?? true, schema: rf.schema ?? { type: "object" } } };
    }
    delete out.response_format;
  }
  // Ensure content types
  if (Array.isArray(out.input)) {
    out.input = out.input.map((turn: any) => ({
      ...turn,
      content: Array.isArray(turn.content)
        ? turn.content.map((c: any) => (c?.type === "text" ? { ...c, type: "input_text" } : c))
        : turn.content,
    }));
  }
  return out;
}
```
- **SDKs**: use the official OpenAI libraries; initialize `OpenAI` client and call `client.responses.create(...)`.
- **SDK versions**: upgrade to a version that supports Responses (Node `openai@latest`; Python `pip install -U openai`) and reinstall dependencies.
- **Streaming**: set `stream: true` and handle SSE events (see docs). Only enable if the original used streaming.
- **Tools**: move Chat Completions function-calling to `tools` with JSON Schema; use `tool_choice` as needed. Provide tool results back via a follow‑up call including a `tool` role item in `input`.
- **Multi‑turn**: manage conversation state in your app; pass prior turns explicitly using `input` items (system/user/assistant/tool). Do not assume server‑side memory by default.
- **Storage**: Default to `store: false` in all cases to reduce overhead and avoid server retention.
- **Data retention & state**:
  - Set `store: false` on all Responses requests.
  - Do not rely on previous message IDs or any server‑stored context; keep state client‑managed and minimize metadata.
- **Deprecations**: do not hardcode dates—link to the official deprecations page in user docs.

See also: `docs/language-recipes/*.md` for idiomatic code diffs.


