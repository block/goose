### Python migration (Completions/Chat → Responses)

Before (legacy):

```python
import openai
openai.api_key = os.getenv("OPENAI_API_KEY")
resp = openai.Completion.create(model="text-davinci-003", prompt="Hello")
print(resp["choices"][0]["text"])
```

After (Responses API):

```python
from openai import OpenAI
client = OpenAI()
resp = client.responses.create(model="gpt-5", input="Hello")
print(resp.output_text)
```

Reasoning and stateless request (encrypted reasoning by default):

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

Mappings:
- prompt → input
- max_tokens → max_output_tokens
- temperature unchanged

Streaming: follow the library's streaming guide if previously used.


