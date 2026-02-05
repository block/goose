# Goose ↔ Proxy ↔ Fake LLM 데이터 규격

## 전체 흐름도

```
┌─────────┐         ┌─────────────────────┐         ┌──────────────┐
│  Goose  │ ──────→ │  Connector Proxy    │ ──────→ │  Fake LLM    │
│         │ OpenAI  │                     │ Fabrix  │  (사내 LLM)  │
│         │ format  │  mode: fabrix       │ format  │              │
│         │         │       or openai     │   or    │              │
│         │ ←────── │                     │ ←────── │              │
│         │ OpenAI  │                     │ OpenAI  │              │
└─────────┘ format  └─────────────────────┘ format  └──────────────┘
```

---

## 1. Goose → Proxy (항상 OpenAI 형식)

### Request

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant..."
    },
    {
      "role": "user",
      "content": "ls 실행해줘"
    },
    {
      "role": "assistant",
      "content": "I'll run ls for you.",
      "tool_calls": [
        {
          "id": "call_abc123",
          "type": "function",
          "function": {
            "name": "developer__shell",
            "arguments": "{\"command\": \"ls\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_abc123",
      "content": "{\"output\": \"file1.txt\\nfile2.txt\", \"success\": true}"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "developer__shell",
        "description": "Execute shell command",
        "parameters": {
          "type": "object",
          "properties": {
            "command": {"type": "string"}
          },
          "required": ["command"]
        }
      }
    }
  ],
  "stream": true,
  "temperature": 0.7
}
```

---

## 2. Proxy → Fake LLM

### Mode: Fabrix (CustomLlmRequest)

```json
{
  "contents": [
    "{\"role\":\"system\",\"content\":\"You are a helpful assistant...\\n\\n# Tool Use Instructions\\n...\"}",
    "{\"role\":\"user\",\"content\":\"ls 실행해줘\"}",
    "{\"role\":\"assistant\",\"content\":\"I'll run ls for you.\\n<tool_call>{\\\"name\\\":\\\"developer__shell\\\",\\\"arguments\\\":{\\\"command\\\":\\\"ls\\\"}}</tool_call>\"}",
    "{\"role\":\"user\",\"content\":\"<tool_response>\\n{\\\"output\\\":\\\"file1.txt\\\\nfile2.txt\\\",\\\"success\\\":true}\\n</tool_response>\"}"
  ],
  "llmId": "gpt-4",
  "isStream": true,
  "llmConfig": {
    "temperature": 0.7,
    "topP": 0.9,
    "maxNewToken": 4096
  }
}
```

**변환 규칙:**
- `tools` → system prompt에 텍스트로 주입 (`# Tool Use Instructions...`)
- `tool_calls` → `<tool_call>{...}</tool_call>` 태그로 변환
- `role: tool` → `role: user` + `<tool_response>...</tool_response>` 래핑

---

### Mode: OpenAI (OpenAI 형식 유지)

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant...\n\n# Tool Use Instructions\n..."
    },
    {
      "role": "user",
      "content": "ls 실행해줘"
    },
    {
      "role": "assistant",
      "content": "I'll run ls for you.\n<tool_call>{\"name\":\"developer__shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>"
    },
    {
      "role": "user",
      "content": "<tool_response>\n{\"output\":\"file1.txt\\nfile2.txt\",\"success\":true}\n</tool_response>"
    }
  ],
  "stream": true,
  "temperature": 0.7
}
```

**변환 규칙:**
- `tools` → system prompt에 텍스트로 주입 (tools 필드 제거)
- `tool_calls` → `<tool_call>{...}</tool_call>` 태그로 변환
- `role: tool` → `role: user` + `<tool_response>...</tool_response>` 래핑

---

## 3. Fake LLM → Proxy

### Mode: Fabrix (CustomLlmResponse)

```json
{
  "id": "chatcmpl-abc123",
  "status": "SUCCESS",
  "content": "Here is the directory listing:",
  "reasoning": "The user wants to run ls. <tool_call>{\"name\":\"developer__shell\",\"arguments\":{\"command\":\"ls -la\"}}</tool_call>",
  "responseCode": "OK",
  "promptToken": 150,
  "completionToken": 50
}
```

**필드:**
| 필드 | 타입 | 필수 | 설명 |
|------|------|------|------|
| id | string | N | 응답 ID |
| status | string | Y | "SUCCESS" 또는 "FAIL" |
| content | string | Y | 응답 텍스트 |
| reasoning | string | N | 추론 과정 (tool_call 포함 가능) |
| responseCode | string | N | 에러 코드 |
| promptToken | int | N | 프롬프트 토큰 수 |
| completionToken | int | N | 완료 토큰 수 |

---

### Mode: OpenAI (Groq 응답 그대로)

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Here is the directory listing:",
        "reasoning": "The user wants to run ls. <tool_call>{\"name\":\"developer__shell\",\"arguments\":{\"command\":\"ls -la\"}}</tool_call>"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 150,
    "completion_tokens": 50,
    "total_tokens": 200
  }
}
```

---

## 4. Proxy → Goose (항상 OpenAI 형식)

### Response (Non-streaming)

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Here is the directory listing:",
        "tool_calls": [
          {
            "id": "call_xyz789",
            "type": "function",
            "function": {
              "name": "developer__shell",
              "arguments": "{\"command\": \"ls -la\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 150,
    "completion_tokens": 50,
    "total_tokens": 200
  }
}
```

**Proxy 변환 규칙:**
1. content에서 `<tool_call>` 파싱 시도
2. 없으면 reasoning에서 `<tool_call>` 파싱 시도
3. 파싱 성공 시:
   - `tool_calls` 필드 추가
   - `finish_reason` = "tool_calls"
   - content에서 `<tool_call>` 태그 제거
4. 파싱 실패 시:
   - `finish_reason` = "stop"
   - content 그대로 유지

---

### Response (Streaming - SSE)

```
data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":123,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":123,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Here is"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":123,"model":"gpt-4","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_xyz","type":"function","function":{"name":"developer__shell","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":123,"model":"gpt-4","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\":"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":123,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":150,"completion_tokens":50}}

data: [DONE]
```

---

## 5. Fabrix Streaming (SSE)

### Fake LLM → Proxy

```
data: {"content":"Here ","event_status":"CHUNK","status":"SUCCESS"}

data: {"content":"is the ","event_status":"CHUNK","status":"SUCCESS"}

data: {"content":"listing.","event_status":"CHUNK","status":"SUCCESS"}

data: {"content":"","event_status":"FINISH","status":"SUCCESS","response_code":"OK","prompt_token":150,"completion_token":50}
```

**CustomSseChunk 필드:**
| 필드 | 타입 | 설명 |
|------|------|------|
| content | string | 청크 텍스트 |
| event_status | string | "CHUNK" 또는 "FINISH" |
| status | string | "SUCCESS" 또는 "FAIL" |
| response_code | string | 에러 코드 (FINISH 시) |
| prompt_token | int | 프롬프트 토큰 (FINISH 시) |
| completion_token | int | 완료 토큰 (FINISH 시) |

---

## 6. Tool Call 형식

### `<tool_call>` 태그 (Proxy 내부)

```xml
<tool_call>
{"name": "developer__shell", "arguments": {"command": "ls -la"}}
</tool_call>
```

### `<tool_response>` 태그 (Proxy 내부)

```xml
<tool_response>
{"output": "file1.txt\nfile2.txt", "success": true}
</tool_response>
```

---

## 7. 에러 응답

### Fabrix 에러

```json
{
  "status": "FAIL",
  "content": "",
  "responseCode": "TIMEOUT"
}
```

### OpenAI 에러

```json
{
  "error": {
    "message": "Connection failed",
    "type": "api_error",
    "code": "connection_error"
  }
}
```

---

## 8. 환경 변수

| 변수 | 설명 | 예시 |
|------|------|------|
| CONNECTOR_MODE | 프록시 모드 | "fabrix" 또는 "openai" |
| CONNECTOR_LLM_URL | LLM 서버 URL | "http://localhost:9090/api/v1/completions" |
| CONNECTOR_LLM_ID | 모델 ID | "gpt-4" |
| CONNECTOR_API_KEY | API 키 (packed 또는 simple) | "client_id<\|>token<\|>user_id" |
| CONNECTOR_PROMPT_LANG | 프롬프트 언어 | "ko" 또는 "en" |
| CONNECTOR_FORCE_NON_STREAM | 강제 non-streaming | "true" |

---

## 9. 파일 위치

| 파일 | 설명 |
|------|------|
| `goose/crates/goose-connector-proxy/src/server.rs` | 프록시 서버 메인 로직 |
| `goose/crates/goose-connector-proxy/src/converter.rs` | OpenAI ↔ Fabrix 변환 |
| `goose/crates/goose-connector-proxy/src/models.rs` | 데이터 모델 정의 |
| `goose/crates/goose-connector-proxy/src/stream.rs` | SSE 스트리밍 처리 |
| `goose/crates/goose-connector-proxy/src/tool_injection.rs` | 도구 프롬프트 주입 및 파싱 |
| `connector/tests/fake_llm_server.py` | 테스트용 Fake LLM 서버 |
