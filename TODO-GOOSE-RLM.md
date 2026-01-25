# TODO: RLM (Recursive Language Models) Implementation for Goose

**Reference Paper**: [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)  
**GitHub Issue**: [#6651](https://github.com/block/goose/issues/6651)

## Overview

Implement RLM support in goose to handle arbitrarily long prompts by treating them as external environment variables that can be programmatically examined, decomposed, and recursively processed through sub-agent calls.

### Key Benefits
- Handle inputs 100x+ beyond normal context windows
- Dramatically outperform base LLMs on long-context tasks
- Maintain comparable or lower cost per query
- Enable processing of 10M+ token inputs

---

## Architecture Summary

```
User Input (large context)
    ↓
Context Store (filesystem/db)
    ↓
REPL Environment (Python)
    ↓
Root Agent (with RLM system prompt)
    ↓
├── Code Execution (filter/chunk context)
├── Sub-Agent Calls (recursive LLM queries)
└── Variable Storage → Final Answer
```

---

## Implementation Checklist

### Phase 1: Core Components

#### 1. Context Storage System
**File**: `goose/toolkit/rlm/context_store.py`

- [ ] Create `ContextStore` class
  - [ ] `store_context(content: str)` - Write context to file
  - [ ] `get_metadata()` - Return length, path, chunk info
  - [ ] `_get_chunk_info(content)` - Calculate chunk boundaries
  - [ ] `read_context()` - Load context from storage
  - [ ] `read_slice(start, end)` - Load partial context

**Key Features**:
- Store context as plain text file in workspace
- Return metadata: `{length, path, chunks}`
- Support chunking by lines, tokens, or bytes

```python
class ContextStore:
    def __init__(self, workspace_path):
        self.workspace = workspace_path
        self.context_file = workspace / "rlm_context.txt"
    
    def store_context(self, content: str) -> dict:
        """Store context and return metadata"""
        # TODO: Implement
        pass
```

---

#### 2. REPL Environment Integration
**File**: `goose/toolkit/rlm/repl_environment.py`

- [ ] Create `RLMEnvironment` class
  - [ ] Initialize with context store and sub-agent factory
  - [ ] Expose `context` variable to Python REPL
  - [ ] Implement `llm_query(prompt, context_chunk)` function
  - [ ] Track variable state across iterations
  - [ ] Handle `FINAL()` and `FINAL_VAR()` tags

**Key Features**:
- Persistent Python REPL across iterations
- `llm_query()` callable from agent's code
- Variables persist between code executions
- Detect final answer tags

```python
class RLMEnvironment:
    def __init__(self, context_store, sub_agent_factory):
        self.store = context_store
        self.sub_agent = sub_agent_factory
        self.variables = {}
        self.repl_globals = {}
    
    def execute_code(self, code: str) -> str:
        """Execute Python code in REPL"""
        # TODO: Implement
        pass
    
    def llm_query(self, prompt: str, context_slice: str = None):
        """Recursively call a sub-agent"""
        # TODO: Implement
        pass
```

---

#### 3. Sub-Agent Factory
**File**: `goose/toolkit/rlm/sub_agent.py`

- [ ] Create `SubAgentFactory` class
  - [ ] Initialize with model name and max recursion depth
  - [ ] `query(prompt, context)` - Call sub-agent
  - [ ] Track recursion depth (default max=1)
  - [ ] Use cheaper model for sub-calls (e.g., gpt-4o-mini)
  - [ ] Handle recursive vs base queries

**Key Features**:
- Use cheaper model for sub-queries (cost optimization)
- Enforce max recursion depth (paper uses depth=1)
- Async support for parallel sub-calls (future optimization)

```python
class SubAgentFactory:
    def __init__(self, model_name="gpt-4o-mini", max_depth=1):
        self.model = model_name
        self.max_depth = max_depth
        self.current_depth = 0
    
    async def query(self, prompt: str, context: str = None):
        """Execute a sub-agent query"""
        # TODO: Implement with depth tracking
        pass
```

---

#### 4. RLM System Prompts
**File**: `goose/toolkit/rlm/prompts.py`

- [ ] Define `RLM_SYSTEM_PROMPT` (adapted from paper Appendix D)
- [ ] Include instructions for:
  - [ ] Accessing `context` variable
  - [ ] Using `llm_query()` for recursive calls
  - [ ] Chunking strategies (aim for ~500K chars per sub-call)
  - [ ] Using `FINAL()` or `FINAL_VAR()` for answers
  - [ ] Code execution patterns (regex, filtering, aggregation)
- [ ] Add model-specific variants (GPT vs Qwen-style)

**Template** (from paper):
```python
RLM_SYSTEM_PROMPT = """
You are tasked with answering a query with associated context. 
You can access, transform, and analyze this context interactively 
in a REPL environment that can recursively query sub-LLMs.

Your context is a {context_type} with {context_total_length} total 
characters, broken into chunks of: {context_lengths}.

The REPL environment provides:
1. A 'context' variable with your input data
2. An 'llm_query(prompt, context_chunk)' function for recursive queries
3. Python execution with 'print()' for viewing results

IMPORTANT: When done, return your answer with:
- FINAL(your answer here) for direct answers
- FINAL_VAR(variable_name) for answers stored in variables

Example strategies:
- Chunk context intelligently (500K chars per sub-call)
- Use regex/code to filter before querying sub-agents
- Store intermediate results in variables
- Batch related queries together
"""
```

---

### Phase 2: Integration with Goose

#### 5. Session/Agent Modifications
**File**: `goose/cli/session.py` or main agent file

- [ ] Add RLM mode detection
  - [ ] Check if input exceeds threshold (default: 100K chars)
  - [ ] CLI flag: `--rlm` to force RLM mode
  - [ ] Auto-enable for large contexts
- [ ] Create `_rlm_process()` method
  - [ ] Initialize context store
  - [ ] Set up REPL environment
  - [ ] Run RLM loop until `FINAL()` tag detected
- [ ] Add cost tracking for recursive calls
- [ ] Implement iteration limit (prevent infinite loops)

```python
class RLMSession(Session):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.rlm_mode = False
        self.context_threshold = 100_000  # chars
        
    async def process_message(self, user_input):
        # Auto-detect large context
        if len(user_input) > self.context_threshold:
            return await self._rlm_process(user_input)
        return await super().process_message(user_input)
    
    async def _rlm_process(self, large_context):
        """Process using RLM strategy"""
        # TODO: Implement RLM loop
        pass
```

---

#### 6. Configuration System
**File**: `profiles.yaml` or config system

- [ ] Add RLM configuration section:
  - [ ] `rlm.enabled` - Enable/disable RLM
  - [ ] `rlm.context_threshold` - Auto-enable threshold (chars)
  - [ ] `rlm.sub_model` - Model for recursive calls
  - [ ] `rlm.max_recursion_depth` - Max depth (default: 1)
  - [ ] `rlm.chunk_size` - Target chunk size (default: 500K)
  - [ ] `rlm.max_iterations` - Safety limit (default: 50)

```yaml
default:
  provider: openai
  processor: gpt-4o
  
  rlm:
    enabled: true
    context_threshold: 100000
    sub_model: gpt-4o-mini
    max_recursion_depth: 1
    chunk_size: 500000
    max_iterations: 50
```

---

#### 7. CLI Enhancements
**File**: `goose/cli/main.py`

- [ ] Add `--rlm` flag to force RLM mode
- [ ] Add `--rlm-threshold` to override threshold
- [ ] Add `--rlm-sub-model` to specify sub-agent model
- [ ] Show RLM status in session info
- [ ] Display recursion depth in logs

```bash
# Usage examples
goose session --rlm
goose session --rlm-threshold 50000
goose session --rlm-sub-model gpt-4o-mini
```

---

### Phase 3: Testing & Validation

#### 8. Unit Tests
**File**: `tests/toolkit/rlm/test_*.py`

- [ ] `test_context_store.py`
  - [ ] Test storing/retrieving context
  - [ ] Test chunk metadata calculation
  - [ ] Test large context handling (1M+ chars)
- [ ] `test_repl_environment.py`
  - [ ] Test code execution
  - [ ] Test variable persistence
  - [ ] Test `llm_query()` function
  - [ ] Test `FINAL()` tag detection
- [ ] `test_sub_agent.py`
  - [ ] Test recursion depth limits
  - [ ] Test sub-agent queries
  - [ ] Test cost tracking

---

#### 9. Integration Tests
**File**: `tests/integration/test_rlm.py`

- [ ] Test with paper's benchmarks:
  - [ ] S-NIAH (needle in haystack)
  - [ ] OOLONG (semantic aggregation)
  - [ ] Simple multi-document QA
- [ ] Test cost vs base model
- [ ] Test context sizes: 100K, 1M, 10M chars
- [ ] Test different chunking strategies
- [ ] Test error handling (max iterations, recursion depth)

---

#### 10. Example Benchmarks
**File**: `examples/rlm_benchmarks/`

Create simple reproducible tests:

- [ ] `needle_in_haystack.py` - S-NIAH style task
  ```python
  # Generate 1M char context with hidden "magic number"
  # Query: "What is the magic number?"
  ```
- [ ] `document_qa.py` - Multi-document question answering
  ```python
  # 100 documents, find answer across multiple docs
  ```
- [ ] `semantic_aggregation.py` - OOLONG style task
  ```python
  # Classify 1000 entries, count by category
  ```

---

### Phase 4: Optimization & Polish

#### 11. Performance Improvements

- [ ] **Async Sub-Agent Calls** (paper Appendix A notes this is critical)
  - [ ] Parallel execution of independent sub-queries
  - [ ] Reduce wall-clock time significantly
- [ ] **Caching**
  - [ ] Cache sub-query results for identical inputs
  - [ ] Cache chunk classifications
- [ ] **Smart Chunking**
  - [ ] Detect natural boundaries (paragraphs, sections)
  - [ ] Use semantic chunking for code repositories

---

#### 12. Documentation

- [ ] **README Updates**
  - [ ] Add RLM section to main README
  - [ ] Explain when to use RLM mode
  - [ ] Show example usage
- [ ] **Tutorial Notebook** (`docs/rlm_tutorial.ipynb`)
  - [ ] Walkthrough of RLM concepts
  - [ ] Compare base model vs RLM
  - [ ] Show cost analysis
- [ ] **API Documentation**
  - [ ] Document all RLM classes and methods
  - [ ] Add docstrings with examples

---

#### 13. Monitoring & Debugging

- [ ] **Cost Tracking**
  - [ ] Track total tokens per RLM query
  - [ ] Track sub-agent call count
  - [ ] Compare to theoretical base model cost
- [ ] **Logging**
  - [ ] Log each RLM iteration
  - [ ] Log sub-agent calls with context
  - [ ] Log final answer extraction
- [ ] **Debug Mode**
  - [ ] `--rlm-debug` flag for verbose output
  - [ ] Visualize recursion tree
  - [ ] Show intermediate variables

---

## Quick Start Implementation Order

**Recommended sequence for fastest MVP**:

1. ✅ Create basic `ContextStore` (Phase 1.1)
2. ✅ Create `RLMEnvironment` with code execution (Phase 1.2)
3. ✅ Add `SubAgentFactory` (Phase 1.3)
4. ✅ Define `RLM_SYSTEM_PROMPT` (Phase 1.4)
5. ✅ Integrate into session/agent (Phase 2.5)
6. ✅ Add config options (Phase 2.6)
7. ✅ Create simple needle-in-haystack test (Phase 3.9)
8. ✅ Test and iterate
9. ✅ Add async sub-calls (Phase 4.11)
10. ✅ Polish and document (Phase 4.12-13)

---

## Key Design Decisions from Paper

### 1. Context as Environment Variable
- **Why**: Prevents context from overwhelming neural network
- **How**: Store in file, expose as Python variable in REPL

### 2. Recursion Depth = 1 (Default)
- **Why**: Paper found depth=1 sufficient for most tasks
- **How**: Root agent uses main model, sub-agents use cheaper model

### 3. Sub-Agent Model Choice
- **GPT Example**: Root=gpt-4o, Sub=gpt-4o-mini
- **Why**: Cost optimization (paper shows comparable quality)

### 4. Chunking Strategy
- **Target**: ~500K chars per sub-call (paper recommendation)
- **Balance**: Large enough to be useful, small enough to avoid context rot

### 5. Answer Detection
- **Tags**: `FINAL(answer)` or `FINAL_VAR(variable_name)`
- **Why**: Clear signal that reasoning is complete

---

## Common Pitfalls to Avoid (from Paper Appendix A)

1. ❌ **Same prompt for all models**
   - Different models need different prompting
   - Add model-specific adjustments

2. ❌ **Insufficient coding ability**
   - Smaller models struggle as RLMs
   - Ensure base model has strong code generation

3. ❌ **Limited output tokens**
   - Thinking models can run out of tokens
   - Set generous output limits

4. ❌ **Synchronous sub-calls only**
   - Makes RLMs very slow
   - Implement async ASAP

5. ❌ **Brittle answer detection**
   - `FINAL()` tag detection can fail
   - Add safeguards and retries

---

## Success Metrics

Track these to validate implementation:

- [ ] **Context Length**: Successfully handle 1M+ char inputs
- [ ] **Accuracy**: Match or exceed base model on test benchmarks
- [ ] **Cost**: Stay within 2x base model cost for equivalent tasks
- [ ] **Speed**: Reasonable completion time (<5min for 1M char inputs)

---

## Resources

- **Paper**: https://arxiv.org/abs/2512.24601
- **GitHub Issue**: https://github.com/block/goose/issues/6651
- **System Prompts**: See paper Appendix D
- **Benchmarks**: OOLONG, S-NIAH, BrowseComp-Plus

---

## Notes

- Start with simple implementation, optimize later
- Test frequently with real long-context tasks
- Monitor costs closely during development
- Consider adding `--dry-run` mode for testing without API calls

---

**Status**: 📋 Ready for implementation  
**Priority**: High (significant capability improvement)  
**Estimated Effort**: 2-4 weeks for MVP, 4-6 weeks for polished release
