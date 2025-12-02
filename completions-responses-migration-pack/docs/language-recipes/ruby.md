### Ruby migration (HTTP/SDK → Responses)

- Use `/v1/responses`.
- Map `prompt` → `input`, `max_tokens` → `max_output_tokens`.

Example (HTTP):

```ruby
uri = URI("https://api.openai.com/v1/responses")
req = Net::HTTP::Post.new(uri)
req["Authorization"] = "Bearer #{ENV["OPENAI_API_KEY"]}"
req["Content-Type"] = "application/json"
req.body = { model: "gpt-5", input: "Hello" }.to_json
```

Reasoning and stateless request:

```ruby
payload = { model: "gpt-5", input: "Hello", store: false, reasoning: { effort: "minimal" } }.to_json
req2 = Net::HTTP::Post.new(uri)
req2["Authorization"] = "Bearer #{ENV["OPENAI_API_KEY"]}"
req2["Content-Type"] = "application/json"
req2.body = payload
```


