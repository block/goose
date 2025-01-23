---
title: Troubleshooting
---

# Troubleshooting
Goose like any system may run into occasional issues. This guide provides solutions for those common problems to ensure a smooth experience.

## Common Issues and How to Handle Them

### **Goose Edits Files**
Goose can and will edit files as part of its workflow. To avoid losing personal changes:
- **Use Version Control**: Stage your personal edits and leave Goose edits unstaged until reviewed.
- **Consider Separate Commits**: Use individual commits for Goose's edits, so you can easily revert them if needed.

---

### **Interrupting Goose**
If Goose is heading in the wrong direction or gets stuck, you can interrupt it:
- **Command**: Press `CTRL+C` to stop Goose, correct its actions, or provide additional information.

---

### **Goose Stuck in a Loop or Unresponsive During Long Sessions**
In rare cases, Goose may enter a "death loop" or become unresponsive during a long session. This is often resolved by ending the current session, and starting a new session.

1. Hold down `Ctrl + C` to cancel
2. Start a new session:
  ```sh
  goose session
  ```
:::tip
For particularly large or complex tasks, consider breaking them into smaller sessions.
:::

---

### Handling Rate Limit Errors
Goose may encounter a `429 error` (rate limit exceeded) when interacting with LLM providers, such as Anthropic's limit of 40,000 input tokens per minute. The recommended solution is to use OpenRouter, OpenRouter automatically manages rate limits and can switch between providers to avoid interruptions. For more help refer to the [Handling LLM Rate Limits with Goose][handling-rate-limits] Guide.

---

### API Errors

Users may run into an error like the one below in the CLI when there are issues with their LLM API tokens, such as running out of credits or incorrect configuration:

```sh
Traceback (most recent call last):
  File "/Users/admin/.local/pipx/venvs/goose-ai/lib/python3.13/site-packages/exchange/providers/utils.py",
line 30, in raise_for_status
    response.raise_for_status()
    ~~~~~~~~~~~~~~~~~~~~~~~~~^^
  File "/Users/admin/.local/pipx/venvs/goose-ai/lib/python3.13/site-packages/httpx/_models.py",
line 829, in raise_for_status
    raise HTTPStatusError(message, request=request, response=self)
httpx.HTTPStatusError: Client error '404 Not Found' for url
'https://api.openai.com/v1/chat/completions'

...
```
This error typically occurs when LLM API credits are expired, or their API key is invalid. To resolve this issue:

1. Check Your API Credits:
    - Log into your LLM provider's dashboard
    - Verify that you have enough credits, if not refill them
2. Verify API Key:
    - Run the following command to reconfigure your API key:
    ```sh
    goose configure
    ```
For detailed steps on updating your LLM provider, refer to the [Installation][installation] Guide.

---

### Need Further Help? 
If you have questions, run into issues, or just need to brainstorm ideas join the [Discord Community][discord]!



[handling-rate-limits]: https://block.github.io/goose/v1/docs/guides/handling-llm-rate-limits-with-goose/
[installation]: http://localhost:3000/goose/v1/docs/installation#update-a-provider
[discord]: https://discord.gg/block-opensource

