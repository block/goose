### C# migration (HTTP/SDK → Responses)

- Endpoint: `/v1/responses`.
- Map `prompt` → `input`, `max_tokens` → `max_output_tokens`.
- Update models to `gpt-5` (or configured default).

Example (HTTP):

```csharp
var req = new HttpRequestMessage(HttpMethod.Post, "https://api.openai.com/v1/responses")
{ Content = new StringContent("{\"model\":\"gpt-5\",\"input\":\"Hello\"}", Encoding.UTF8, "application/json") };
req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", Environment.GetEnvironmentVariable("OPENAI_API_KEY"));
```

Reasoning and stateless request:

```csharp
var payload = "{\"model\":\"gpt-5\",\"input\":\"Hello\",\"store\":false,\"reasoning\":{\"effort\":\"minimal\"}}";
var req2 = new HttpRequestMessage(HttpMethod.Post, "https://api.openai.com/v1/responses")
{ Content = new StringContent(payload, Encoding.UTF8, "application/json") };
req2.Headers.Authorization = new AuthenticationHeaderValue("Bearer", Environment.GetEnvironmentVariable("OPENAI_API_KEY"));
```


