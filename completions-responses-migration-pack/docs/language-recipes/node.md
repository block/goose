### Node.js migration (Completions/Chat → Responses)

Before (legacy):

```js
// openai v3 style
const { Configuration, OpenAIApi } = require("openai");
const openai = new OpenAIApi(new Configuration({ apiKey: process.env.OPENAI_API_KEY }));
const res = await openai.createCompletion({ model: "text-davinci-003", prompt: "Hello" });
console.log(res.data.choices[0].text);
```

After (Responses API):

```js
import OpenAI from "openai";
const client = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
const res = await client.responses.create({ model: "gpt-5", input: "Hello" });
console.log(res.output_text);
```

Reasoning and stateless request (encrypted reasoning by default):

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

Streaming (if originally used):

```js
const stream = await client.responses.create({ model: "gpt-5", input: "stream me", stream: true });
for await (const event of stream) {
  if (event.delta) process.stdout.write(event.delta);
}
```
Built‑in tool example (web search preview):

```js
const res = await client.responses.create({
  model: "gpt-5",
  tools: [{ type: "web_search_preview" }],
  input: "What was a positive news story from today?"
});
console.log(res.output_text);
```

Image input example:

```js
const res = await client.responses.create({
  model: "gpt-5",
  input: [
    { role: "user", content: "Describe this image" },
    { role: "user", content: [ { type: "input_image", image_url: "https://example.com/image.jpg" } ] }
  ]
});
console.log(res.output_text);
```

Mappings:
- prompt → input
- max_tokens → max_output_tokens
- temperature unchanged
- response_format (chat) → text.format (responses). Example for JSON output:

```js
const schema = {
  type: "object",
  properties: { answer: { type: "string" } },
  required: ["answer"],
  additionalProperties: false
};
const res = await client.responses.create({
  model: "gpt-5",
  input: "Return JSON",
  text: { format: { type: "json_schema", name: "Output", strict: true, schema } }
});
const obj = JSON.parse(res.output_text);
```

Multi‑turn items shape (Responses):

```js
const input = [
  { role: "system", content: [{ type: "input_text", text: "You are helpful." }] },
  { role: "user", content: [{ type: "input_text", text: "Hi" }] },
  { role: "assistant", content: [{ type: "output_text", text: "Hello!" }] }
];
const res = await client.responses.create({ model: "gpt-5", input });
```

Minimal Next.js route example (mapping legacy fields):

```ts
import { NextResponse } from "next/server";
import OpenAI from "openai";

export async function POST(req: Request) {
  const body = await req.json();
  const client = new OpenAI();

  const payload: any = { model: body.model ?? "gpt-5" };
  // reasoning effort for GPT-5 based on prior model family
  if (!body?.reasoning?.effort) {
    const prev = body?.model || "";
    const effort = /^o[\w-]*/i.test(prev) ? "medium" : "minimal";
    payload.reasoning = { effort };
  }
  // input mapping (string or items)
  if (body.input) payload.input = body.input;
  if (!payload.input && Array.isArray(body.messages)) {
    payload.input = body.messages.map((m: any) => ({
      role: m.role,
      content: (m.content ?? m.text ?? "").trim()
        ? [{ type: "input_text", text: typeof m.content === "string" ? m.content : m.content?.[0]?.text ?? "" }]
        : []
    }));
  }
  // parameter mapping
  if (body.max_output_tokens !== undefined) payload.max_output_tokens = body.max_output_tokens;
  if (body.max_tokens !== undefined && payload.max_output_tokens === undefined) payload.max_output_tokens = body.max_tokens;
  if (body.temperature !== undefined) payload.temperature = body.temperature;
  // response_format → text.format
  if (body.response_format && !payload.text?.format) {
    const rf = body.response_format;
    if (rf === "json") {
      payload.text = { format: { type: "json_schema", json_schema: { name: "Output", strict: true, schema: { type: "object" } } } };
    } else if (typeof rf === "object") {
      payload.text = { format: { type: "json_schema", json_schema: { name: rf.name ?? "Output", strict: rf.strict ?? true, schema: rf.schema ?? { type: "object" } } } };
    }
  }

  const res = await client.responses.create(payload);
  return NextResponse.json(res);
}
```


