# LangGraph Methods Report

**Project:** AI Agent Core
**LangGraph:** 0.6.6 | **Checkpoint:** langgraph-checkpoint-postgres 2.0.23

---

## Graph Structure

```
START → extract_user_query → retrieve_rag_documents → plan_troubleshooting_steps
      → classify_user_intent → process_user_intent → generate_model_response → END
```

---

## Methods Used

### Imports

| Import | File |
|--------|------|
| `StateGraph`, `START`, `END` | `app/infrastructure/agent/graph.py:13` |
| `add_messages` | `app/infrastructure/agent/initial_state.py:10` |
| `AsyncPostgresSaver` | `app/main.py:23` |

### Graph Construction

| Method | Location |
|--------|----------|
| `StateGraph(State)` | `graph.py:38` |
| `.add_node()` | `graph.py:41-46` |
| `.add_edge()` | `graph.py:49-55` |
| `.compile(checkpointer)` | `graph.py:58` |

### Graph Execution

| Method | Location | Mode |
|--------|----------|------|
| `.ainvoke()` | `agent.py:83` | Sync execution |
| `.astream()` | `agent.py:124` | Streaming (`stream_mode="messages"`) |
| `.aget_state()` | `agent.py:69,105` / `message.py:93,224,274` | State retrieval |

### Checkpointing

| Method | Location |
|--------|----------|
| `AsyncPostgresSaver(conn)` | `main.py:73` |
| `.setup()` | `main.py:77` |

### State Reducers

| Reducer | Field |
|---------|-------|
| `add_messages` (langgraph) | `messages` |
| `add_last_3` (custom) | `user_intents` |

---

## Instrumentation Targets

| Target | File | Metrics |
|--------|------|---------|
| `ainvoke()` | `agent.py:83` | Latency, tokens |
| `astream()` | `agent.py:124` | TTFT, chunk count |
| `aget_state()` | `agent.py:69` | Retrieval latency |
| `AsyncPostgresSaver` | `main.py:73` | Write latency |
| Node functions (6) | `graph.py:41-46` | Per-node execution time |
