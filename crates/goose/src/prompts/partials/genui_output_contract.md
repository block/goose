## Output contract (json-render)

### Required

- Output **only** a json-render spec in one of these formats:
  - **JSONL** RFC 6902 JSON Patch operations (**one JSON object per line**), or
  - **Nested JSON** with a `root` element tree.
- **Do not include extra prose** before or after the spec.

### Markdown fences

- Markdown fences are **optional**.
- If you use a fence, it **MUST** be a `json-render` fence and it **MUST** be closed:

```text
```json-render
...
```
```

### Streaming guidance (JSONL)

- Start with `/root`.
- Then stream `/elements` and `/state` patches interleaved so the UI fills in progressively.
- If you reference `$state`, `$bindState`, `$bindItem`, `$item`, `$index`, or `repeat`, you **MUST** include the corresponding `/state` patches.
