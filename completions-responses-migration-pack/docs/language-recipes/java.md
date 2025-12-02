### Java migration (HTTP/SDK → Responses)

- Switch base path from `/v1/completions` to `/v1/responses`.
- Map fields: `prompt` → `input`, `max_tokens` → `max_output_tokens`.
- Update response parsing to Responses schema (aggregate `output_text` when available in SDKs).

Example (HTTP):

```java
HttpRequest req = HttpRequest.newBuilder()
  .uri(URI.create("https://api.openai.com/v1/responses"))
  .header("Authorization", "Bearer " + System.getenv("OPENAI_API_KEY"))
  .header("Content-Type", "application/json")
  .POST(HttpRequest.BodyPublishers.ofString("{\"model\":\"gpt-5\",\"input\":\"Hello\"}"))
  .build();
```

Reasoning and stateless request:

```java
String payload = "{\"model\":\"gpt-5\",\"input\":\"Hello\",\"store\":false,\"reasoning\":{\"effort\":\"minimal\"}}";
HttpRequest req2 = HttpRequest.newBuilder()
  .uri(URI.create("https://api.openai.com/v1/responses"))
  .header("Authorization", "Bearer " + System.getenv("OPENAI_API_KEY"))
  .header("Content-Type", "application/json")
  .POST(HttpRequest.BodyPublishers.ofString(payload))
  .build();
```


