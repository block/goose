# Framework Instrumentation Guide for OpenLIT

This guide provides a comprehensive, step-by-step process for adding new framework instrumentations or updating existing ones in OpenLIT. It's based on lessons learned from successful instrumentations including OpenAI Agents, Haystack, and others, ensuring consistency, performance, and competitive advantages across all framework integrations.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Phase 1: Competitive Analysis](#phase-1-competitive-analysis)
3. [Phase 2: Framework Analysis](#phase-2-framework-analysis)
4. [Phase 3: Implementation Strategy](#phase-3-implementation-strategy)
5. [Phase 4: Code Implementation](#phase-4-code-implementation)
6. [Phase 5: Testing & Validation](#phase-5-testing--validation)
7. [Phase 6: Optimization](#phase-6-optimization)
8. [Phase 7: Post-Change Cleanup](#phase-7-post-change-cleanup)
9. [Code Standards & Patterns](#code-standards--patterns)
10. [Quality Checklist](#quality-checklist)

## Prerequisites

Before starting, ensure you have:
- OpenLIT development environment set up
- Target framework installed and working examples
- Access to competitor repositories for analysis
- Understanding of OpenTelemetry concepts

**CRITICAL**: Always use the venv in the root directory and test with test.py:
```bash
# ALWAYS use the venv in project root
source venv/bin/activate

# ALWAYS name your test file test.py (mandatory naming convention)
python test.py

# Alternative approach with explicit sys.path (for test.py):
sys.path.insert(0, "sdk/python/src")
```

**MANDATORY Testing Standards:**
- Test file must be named `test.py` (not test_framework.py or similar)
- Always use `source venv/bin/activate` first
- Place test.py in project root directory
- Use sys.path.insert(0, "sdk/python/src") in test.py

## Phase 1: Competitive Analysis

### Step 1.1: Clone and Research Competitor Implementations

**Primary Competitors:**
- [OpenInference](https://github.com/Arize-ai/openinference/tree/main/python/instrumentation)
- [OpenLLMetry](https://github.com/traceloop/openllmetry/tree/main/packages)
- [AgentOps](https://github.com/AgentOps-AI/agentops)
- [LangFuse](https://github.com/langfuse/langfuse) (Not OpenTelemetry native)
- [LangSmith](https://github.com/langchain-ai/langsmith-sdk)

**Research Process:**
```bash
# Clone competitor repositories for deep analysis
mkdir competitive_analysis && cd competitive_analysis
git clone https://github.com/Arize-ai/openinference.git
git clone https://github.com/traceloop/openllmetry.git
git clone https://github.com/AgentOps-AI/agentops.git
git clone https://github.com/langfuse/langfuse.git
git clone https://github.com/langchain-ai/langsmith-sdk.git

# Navigate to framework-specific instrumentations
cd openinference/python/instrumentation/openinference-instrumentation-{framework}
cd openllmetry/packages/openllmetry-instrumentation-{framework}
```

**Deep Analysis Checklist:**
- [ ] **Integration Pattern**: Function wrapping vs native integration (like TracingProcessor)
- [ ] **Span Structure**: How many spans do they create and why?
- [ ] **🚨 CRITICAL: Span Hierarchy**: Do they maintain proper parent-child relationships? (MOST IMPORTANT)
- [ ] **Threading Context**: How do they handle ThreadPoolExecutor and async contexts?
- [ ] **Span Naming**: What naming convention do they use?
- [ ] **Attributes**: What attributes do they capture? Check against semantic conventions
- [ ] **Content Capture**: Do they capture input/output content with MIME types?
- [ ] **Business Intelligence**: Do they track cost, tokens, performance metrics?
- [ ] **Error Handling**: How do they handle framework version differences?
- [ ] **Performance**: How much overhead do they add?
- [ ] **Coverage**: Which framework operations do they instrument?

**⚠️ SPAN HIERARCHY IS CRITICAL**: This is the #1 issue that breaks observability. Frameworks often use ThreadPoolExecutor or async operations that break OpenTelemetry context propagation, resulting in "all root spans" instead of proper parent-child relationships.

### Step 1.2: Document Competitive Gaps and OpenLIT Advantages

Create detailed comparison:

| Feature | OpenInference | OpenLLMetry | OpenLIT Target |
|---------|---------------|-------------|----------------|
| Integration Method | Function wrapping | Function wrapping | Native (if available) |
| Span Count | X spans | Y spans | Z+ spans (comprehensive) |
| Business Intelligence | None/Basic | None/Basic | **Complete** (cost, tokens, metrics) |
| Content Capture | Basic | Basic | **Enhanced** (MIME types, structured) |
| Cross-System Tracing | Limited | Limited | **Full** (links to LLM providers) |
| Error Resilience | Basic | Basic | **Robust** (graceful degradation) |
| Semantic Conventions | Standard | Standard | **Extended** (custom attributes) |

**Key Insight**: OpenLIT's competitive advantage is **business intelligence** and **enhanced observability**.

### Step 1.2: Instrumentation Structure Patterns

**FLEXIBLE file structures based on framework complexity:**

```python
# PATTERN 1: Single File (Simple frameworks)
# Example: Simple utilities, basic wrappers
framework_name/
├── __init__.py      # Instrumentation setup + all logic
└── (optional utils if needed)

# PATTERN 2: Processor Pattern (OpenAI Agents style)
# Example: Pipeline/processing-based frameworks
framework_name/
├── __init__.py      # Instrumentation setup
└── processor.py     # All processing logic, context handling

# PATTERN 3: Callback Pattern (LangChain style)
# Example: Frameworks with built-in callback systems
framework_name/
├── __init__.py      # Instrumentation setup
└── callback_handler.py # Framework's callback interface

# PATTERN 4: RECOMMENDED 4-File Structure (Mem0, CrewAI, Pydantic AI)
# Example: Complex frameworks needing performance optimization
framework_name/
├── __init__.py                  # Instrumentation setup with separated SYNC/ASYNC methods
├── framework_name.py            # Sync wrappers with threading context fixes  
├── async_framework_name.py      # Async wrappers
└── utils.py                     # Shared utilities with context caching and __slots__

**🎯 PREFERRED STRUCTURE**: Use Pattern 4 for any framework with threading issues or performance needs.

```

### Step 1.3: Threading Context Propagation Issues (CRITICAL)

**🚨 MOST COMMON FAILURE**: Frameworks using ThreadPoolExecutor break OpenTelemetry context propagation.

**Example Problem (like mem0):**
```python
# mem0/memory/main.py - Line 257-261
with concurrent.futures.ThreadPoolExecutor() as executor:
    future1 = executor.submit(self._add_to_vector_store, messages, ...)
    future2 = executor.submit(self._add_to_graph, messages, ...)
    # ❌ This breaks context propagation - results in "all root spans"
```

**OpenLIT Solution Pattern:**
```python
# In utils.py - Threading context fix
def patch_concurrent_futures_context(span_context):
    """Patch ThreadPoolExecutor to propagate OpenTelemetry context."""
    original_submit = concurrent.futures.ThreadPoolExecutor.submit
    
    def patched_submit(self, fn, *args, **kwargs):
        # Capture current context and propagate it
        current_context = context.get_current()
        
        def context_wrapper(*args, **kwargs):
            token = context.attach(span_context)
            try:
                return fn(*args, **kwargs)
            finally:
                context.detach(token)
        
        return original_submit(self, context_wrapper, *args, **kwargs)
    
    concurrent.futures.ThreadPoolExecutor.submit = patched_submit
    return lambda: setattr(concurrent.futures.ThreadPoolExecutor, "submit", original_submit)

# In framework_name.py - Apply threading fix  
def framework_wrap(gen_ai_endpoint, ...):
    def wrapper(wrapped, instance, args, kwargs):
        with tracer.start_as_current_span(gen_ai_endpoint, kind=span_kind, context=current_context) as span:
            span_context = set_span_in_context(span, context=current_context)
            
            # ✅ Apply threading context fix for CLIENT operations
            if span_kind == SpanKind.CLIENT:
                restore_patch = patch_concurrent_futures_context(span_context)
                try:
                    response = wrapped(*args, **kwargs)
                finally:
                    restore_patch()
            else:
                response = wrapped(*args, **kwargs)
```

**🔧 When to Apply Threading Fixes:**
- Framework uses `concurrent.futures.ThreadPoolExecutor`
- Framework uses `asyncio.run_in_executor`
- You see "all root spans" in your hierarchy test
- Child operations (OpenAI, Qdrant, etc.) appear as separate root spans

### Step 1.4: Performance Optimization Patterns

**CRITICAL**: Study existing OpenLIT implementations for optimization patterns:

```python
# Study these proven patterns in existing instrumentations:
# - Mem0: threading context propagation, __slots__ optimization
# - CrewAI: excellent agent patterns, lifecycle management
# - OpenAI Agents: great tool handling (processor.py pattern)
# - LangChain/LlamaIndex: mature framework patterns, utils.py caching
# - Pydantic AI: context caching with PydanticAIInstrumentationContext

# Example: Optimized context caching with __slots__
class FrameworkInstrumentationContext:
    """Context object to cache expensive extractions."""
    
    __slots__ = ("instance", "args", "kwargs", "version", "environment", 
                 "application_name", "_agent_name", "_model_name", "_tools", "_messages")
    
    def __init__(self, instance, args, kwargs, version, environment, application_name):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name
        
        # Cache expensive operations with lazy loading
        self._agent_name = None
        self._model_name = None
        self._tools = None
        self._messages = None
    
    @property
    def agent_name(self) -> str:
        """Lazy-loaded with caching."""
        if self._agent_name is None:
            self._agent_name = self._extract_agent_name()
        return self._agent_name
```

## Phase 2: Framework Analysis

### Step 2.1: Explore Framework's Built-in Capabilities

**CRITICAL**: Before implementing, check if the framework has built-in tracing/monitoring:

```python
# Example: Discover OpenAI Agents has native tracing
import {framework}

# Look for tracing, monitoring, observability features
# Check documentation for:
# - Built-in tracing systems
# - Processor patterns  
# - Event hooks
# - Monitoring callbacks

# Example: OpenAI Agents has TracingProcessor
from agents import TracingProcessor, set_trace_processors
```

**Framework Integration Decision Tree:**
1. **Native Integration Available** (e.g., OpenAI Agents TracingProcessor)
   - ✅ Use native integration for perfect hierarchy
   - ✅ Extends framework's built-in system
   - ✅ Better performance and reliability

2. **Function Wrapping Required** (most frameworks)
   - ✅ Standard approach for frameworks without native tracing
   - ⚠️ Requires careful hierarchy management

### Step 2.2: Clone and Study Target SDK (CRITICAL)

**ALWAYS clone the actual SDK you're instrumenting:**

### Step 2.3: Semantic Conventions Are CRITICAL

**NEVER hardcode strings - ALWAYS use SemanticConvention:**

```python
# ❌ WRONG - will be rejected in code review:
span.set_attribute("gen_ai.operation.name", "chat")
span.set_attribute("gen_ai.system", "pydantic_ai")

# ✅ CORRECT - must use semantic conventions:
from openlit.semcov import SemanticConvention

span.set_attribute(SemanticConvention.GEN_AI_OPERATION, SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT)
span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_PYDANTIC_AI)

# Check existing constants first:
# - GEN_AI_OPERATION_TYPE_* 
# - GEN_AI_AGENT_LIFECYCLE_PHASE_*
# - GEN_AI_SYSTEM_* (add new ones if needed)

# If needed, add to semcov.py:
class SemanticConvention:
    # Add framework-specific constants
    GEN_AI_SYSTEM_PYDANTIC_AI = "pydantic_ai"
    GEN_AI_AGENT_LIFECYCLE_PHASE_GRAPH_EXECUTION = "graph_execution"
```

### Step 2.4: Map Framework Operations

**Workflow vs Component Operations:**
```python
# Example for any framework
WORKFLOW_OPERATIONS = [
    # High-level operations users care about in production
    "agent.run_sync",     # OpenAI Agents
    "pipeline.run",       # Haystack  
    "workflow.execute"    # Generic
]

COMPONENT_OPERATIONS = [
    # Detailed operations for debugging (detailed_tracing=True)
    "retriever.retrieve",
    "generator.generate", 
    "embedder.embed",
    "tool.call"
]
```

### Step 2.3: Test Framework Methods and Patterns

**Method Discovery Script:**
```python
# test_framework_exploration.py
import {framework}

# Discover available methods and attributes
obj = SomeFrameworkClass()
print("Available methods:", [m for m in dir(obj) if not m.startswith('_')])

# Test built-in monitoring
if hasattr(obj, 'trace') or hasattr(obj, 'monitor'):
    print("Built-in tracing available!")
    
# Check for callback patterns
if hasattr(obj, 'add_callback') or hasattr(obj, 'on_event'):
    print("Event system available!")
```

## Phase 3: Implementation Strategy

### Step 3.1: Choose Integration Pattern

**Pattern 1: Native Integration (Preferred when available)**
```python
# Example: OpenAI Agents TracingProcessor
class OpenLITTracingProcessor(TracingProcessor):
    def on_trace_start(self, trace): pass
    def on_span_start(self, span): pass  
    def on_span_end(self, span): pass
```

**Pattern 2: Function Wrapping (Standard)**
```python
# Traditional OpenLIT pattern
wrap_function_wrapper("module", "Class.method", wrapper)
```

### Step 3.2: Plan Span Naming Convention

**CRITICAL**: Follow consistent naming pattern:
```python
# Standard format: "{operation_type} {operation_name}"
"agent Triage agent"      # operation_type=agent, operation_name=Triage agent
"chat gpt-4o"            # operation_type=chat, operation_name=gpt-4o  
"retrieve documents"      # operation_type=retrieve, operation_name=documents
"generate response"       # operation_type=generate, operation_name=response
```

### Step 3.3: Plan Semantic Conventions Usage

**Extend semcov.py when needed:**
```python
# It's OK to add new semantic conventions for better attributes
class SemanticConvention:
    # Add framework-specific conventions
    GEN_AI_AGENT_DESCRIPTION = "gen_ai.agent.description"
    GEN_AI_AGENT_VERSION = "gen_ai.agent.version"
    GEN_AI_WORKFLOW_TYPE = "gen_ai.workflow.type"
```

## Phase 4: Code Implementation

### Step 4.1: Create Span Hierarchy Test First (MANDATORY)

**🚨 CRITICAL**: Create test.py (mandatory name) with span hierarchy analysis BEFORE any implementation:

```python
# test.py - MANDATORY filename, must be in project root
#!/usr/bin/env python3
"""
MANDATORY span hierarchy test for framework instrumentation.
This MUST be created first and MUST verify proper parent-child relationships.
"""

import sys
import os

sys.path.insert(0, "sdk/python/src")  # MANDATORY path setup

import openlit
import logging

# Minimal logging to focus on span hierarchy
logging.getLogger().setLevel(logging.ERROR)

def test_framework_hierarchy():
    """
    🚨 CRITICAL TEST: Verify span hierarchy is correct, not "all root spans"
    """
    print("🧠 Framework Instrumentation Test")
    print("=" * 40)

    # Initialize OpenLIT - use venv: source venv/bin/activate
    print("✅ Initializing OpenLIT...")
    openlit.init(detailed_tracing=True)

    # Import and use framework
    print("✅ Testing framework operations...")
    from your_framework import YourClass

    instance = YourClass()

    # Test operations that should create proper hierarchy
    result1 = instance.main_operation("test data", user_id="test_user")
    print(f"✅ Main operation result: {result1}")

    result2 = instance.secondary_operation("search query", user_id="test_user", limit=1)
    print(f"✅ Secondary operation result: {len(result2) if result2 else 0} items")

    print("\n🎯 EXPECTED HIERARCHY:")
    print("🔸 ROOT: main_operation")
    print("  ↳ internal_operation_1")
    print("    ↳ openai_call (or similar)")
    print("    ↳ database_operation")
    print("🔸 ROOT: secondary_operation")  
    print("  ↳ internal_operation_2")
    print("    ↳ embedding_call")

    print("\n⚠️  VERIFY: Check console output above for proper parent-child relationships")
    print("❌ If you see 'all root spans', you have threading context propagation issues!")

if __name__ == "__main__":
    test_framework_hierarchy()
```

**🔧 SPAN HIERARCHY DEBUGGING COMMANDS:**

```bash
# Run test.py and analyze hierarchy
source venv/bin/activate  # MANDATORY first step
python test.py > spans.json 2>&1

# Parse and analyze hierarchy (create this script)
python3 -c "
import json
# ... hierarchy parsing script from mem0 experience ...
# Shows proper tree structure or identifies 'all root spans' issue
"
```

**⚠️ HIERARCHY TESTING IS THE MOST IMPORTANT STEP**: 
- If hierarchy is broken (all root spans), nothing else matters
- This is the #1 failure mode in framework instrumentation  
- Threading context propagation issues MUST be caught early

### Step 4.2: Implement Based on Integration Pattern

**For Native Integration:**
```python
# processor.py - When framework has built-in tracing
class OpenLITTracingProcessor(FrameworkProcessor):
    def on_span_start(self, span):
        # Create OpenTelemetry span with proper naming
        span_name = self._get_span_name(span)  # {operation_type} {operation_name}
        otel_span = self._tracer.start_span(span_name, kind=SpanKind.CLIENT)
        
        # Set semantic convention attributes
        self._set_common_attributes(otel_span, span)
        
    def _get_span_name(self, span):
        operation_type = self._get_operation_type(span.span_data)
        operation_name = self._extract_operation_name(span.span_data)
        return f"{operation_type} {operation_name}"
```

**For Function Wrapping:**
```python
# Follow existing 4-file structure
# __init__.py, sync_wrapper.py, async_wrapper.py, utils.py
```

### Step 4.3: Implement Comprehensive Attributes

**Use semantic conventions extensively:**
```python
def _set_span_attributes(self, span, data):
    # Standard framework attributes
    span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, "framework_name")
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION_NAME, operation_name)
    
    # Model information (critical for business intelligence)
    model = self._extract_model_info(data)
    if model:
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model)
    
    # Content capture with MIME types (OpenLIT enhancement)
    if self._capture_message_content:
        self._capture_input_output_with_mime_types(span, data)
    
    # Business intelligence attributes
    self._capture_token_usage(span, data)
    self._capture_cost_metrics(span, data)
    
    # Framework-specific attributes
    if hasattr(data, 'agent_name'):
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, data.agent_name)
```

## Phase 5: Testing & Validation

### Step 5.1: Create Comprehensive Test Suite

**Test Categories:**
1. **Span Hierarchy Test** (most important)
2. **Competitive Comparison Test** (CRITICAL)
3. **Performance Benchmarking Test**
4. **Mock Framework Test** (when real framework unavailable)
5. **Error Resilience Test**

**Span Hierarchy Validation:**
```python
def test_span_hierarchy():
    """Validate proper parent-child relationships and span naming"""
    spans = run_instrumentation_test()
    
    # Validate hierarchy structure
    assert len(spans) > 0, "Must generate spans"
    
    # Check root span exists
    root_spans = [s for s in spans if s['parent_id'] is None]
    assert len(root_spans) == 1, "Must have exactly one root span"
    
    # Validate span naming convention
    for span in spans:
        assert ' ' in span['name'], f"Span name '{span['name']}' should follow '{{operation_type}} {{operation_name}}' format"
    
    # Check semantic conventions
    for span in spans:
        attrs = span['attributes']
        assert 'gen_ai.system' in attrs, "Must have gen_ai.system attribute"
```

### Step 5.2: Competitive Validation

**Critical Test - OpenLIT vs Competitors:**
```python
def test_competitive_superiority():
    """Validate OpenLIT generates more/better spans than competitors"""
    
    # Test OpenLIT
    openlit_spans = test_openlit_instrumentation()
    
    # Test competitors (with error handling)
    try:
        competitor1_spans = test_competitor_instrumentation("openinference")
        competitor2_spans = test_competitor_instrumentation("openllmetry")
        competitor3_spans = test_competitor_instrumentation("agentops")
    except Exception as e:
        print(f"Competitor failed: {e}")
        competitor1_spans = competitor2_spans = competitor3_spans = 0
    
    # Validate OpenLIT superiority
    assert openlit_spans > 0, "OpenLIT must generate spans"
    print(f"OpenLIT: {openlit_spans} spans (with business intelligence)")
    print(f"OpenInference: {competitor1_spans} spans") 
    print(f"OpenLLMetry: {competitor2_spans} spans")
    print(f"AgentOps: {competitor3_spans} spans")
    
    # OpenLIT should provide comprehensive coverage
    assert openlit_spans >= max(competitor1_spans, competitor2_spans, competitor3_spans)
```

### Step 5.3: Performance Benchmarking

**Critical Test - Performance vs Competitors:**
```python
def benchmark_instrumentation_performance():
    """Compare instrumentation overhead across competitors."""
    
    import time
    
    # Test scenarios
    test_cases = [
        "single_agent_simple_task",
        "multi_agent_conversation", 
        "tool_heavy_workflow",
        "high_frequency_requests"
    ]
    
    competitors = ["openlit", "openinference", "openllmetry", "agentops"]
    
    results = {}
    for competitor in competitors:
        results[competitor] = {}
        
        for test_case in test_cases:
            # Baseline without instrumentation
            start = time.time()
            run_test_case(test_case, instrumentation=None)
            baseline = time.time() - start
            
            # With competitor instrumentation
            start = time.time()
            run_test_case(test_case, instrumentation=competitor)
            instrumented = time.time() - start
            
            overhead = ((instrumented - baseline) / baseline) * 100
            results[competitor][test_case] = {
                "overhead_percent": overhead,
                "baseline_ms": baseline * 1000,
                "instrumented_ms": instrumented * 1000
            }
    
    # Validate OpenLIT performance
    openlit_avg_overhead = sum(results["openlit"][tc]["overhead_percent"] for tc in test_cases) / len(test_cases)
    assert openlit_avg_overhead < 10, f"OpenLIT overhead too high: {openlit_avg_overhead:.2f}%"
    
    return results
```

### Step 5.3: Mock Testing When Framework Unavailable

**Create realistic mock scenarios:**
```python
def test_with_mocks():
    """Test instrumentation when real framework isn't available"""
    tracer = trace.get_tracer(__name__)
    
    # Create realistic span hierarchy manually
    with tracer.start_as_current_span("agent Agent workflow") as root:
        root.set_attribute("gen_ai.agent.name", "Agent workflow")
        
        with tracer.start_as_current_span("agent Triage agent") as agent:
            agent.set_attribute("gen_ai.agent.name", "Triage agent")
            
            with tracer.start_as_current_span("chat gpt-4o") as chat:
                chat.set_attribute("gen_ai.request.model", "gpt-4o")
```

## Phase 6: Optimization

### Step 6.1: Context Caching for Performance

**Implement context caching pattern (learned from Pydantic AI optimization):**

```python
class FrameworkInstrumentationContext:
    """Context object to cache expensive extractions and reduce performance overhead."""
    
    def __init__(self, instance, args, kwargs, version, environment, application_name):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name
        
        # Cache expensive operations with lazy loading
        self._agent_name = None
        self._model_name = None
        self._server_info = None
        self._messages = None
        self._tools = None
        self._model_params = None
        
    @property
    def agent_name(self) -> str:
        """Get agent name with caching - avoids repeated extraction."""
        if self._agent_name is None:
            self._agent_name = getattr(self.instance, 'name', None) or "default_agent"
        return self._agent_name
    
    @property
    def model_name(self) -> str:
        """Get model name with caching."""
        if self._model_name is None:
            if hasattr(self.instance, 'model') and hasattr(self.instance.model, 'model_name'):
                self._model_name = str(self.instance.model.model_name)
            else:
                self._model_name = "unknown"
        return self._model_name

def set_span_attributes(span, operation_name: str, ctx: FrameworkInstrumentationContext, 
                       lifecycle_phase: Optional[str] = None,
                       additional_attrs: Optional[Dict[str, Any]] = None):
    """Optimized attribute setting with context caching."""
    
    # Set core attributes using cached context
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_name)
    span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_FRAMEWORK)
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, ctx.agent_name)
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, ctx.model_name)
    
    # Set environment attributes
    span.set_attribute(DEPLOYMENT_ENVIRONMENT, ctx.environment)
    span.set_attribute(SERVICE_NAME, ctx.application_name)
    span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, ctx.version)
    
    # Set lifecycle phase if provided
    if lifecycle_phase:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE, lifecycle_phase)
    
    # Set additional attributes
    if additional_attrs:
        for key, value in additional_attrs.items():
            span.set_attribute(key, value)
```

### Step 6.2: Performance Validation

**Measure instrumentation overhead:**
```python
def benchmark_performance():
    """Measure instrumentation performance impact"""
    import time
    
    # Test without instrumentation
    start = time.time()
    run_framework_operations(count=100)
    baseline = time.time() - start
    
    # Test with instrumentation
    openlit.init()
    start = time.time()
    run_framework_operations(count=100)
    instrumented = time.time() - start
    
    overhead = ((instrumented - baseline) / baseline) * 100
    print(f"Instrumentation overhead: {overhead:.2f}%")
    assert overhead < 10, "Overhead should be under 10%"
```

### Step 6.2: Business Intelligence Validation

**Verify OpenLIT's competitive advantages:**
```python
def test_business_intelligence():
    """Validate comprehensive business metrics capture"""
    spans = run_instrumentation_test()
    
    business_attributes = [
        'gen_ai.usage.input_tokens',
        'gen_ai.usage.output_tokens',
        'gen_ai.client.operation.duration',
        'gen_ai.request.model'
    ]
    
    for span in spans:
        attrs = span['attributes']
        found_attrs = [attr for attr in business_attributes if attr in attrs]
        print(f"Span '{span['name']}': {len(found_attrs)}/{len(business_attributes)} business attributes")
```

## Phase 7: Post-Change Cleanup

**CRITICAL**: Always run comprehensive cleanup after implementation:

### Step 7.1: Automated Code Quality Check

```bash
#!/bin/bash
# cleanup_instrumentation.sh
FRAMEWORK_DIR="src/openlit/instrumentation/$1"

echo "🧹 Cleaning up $FRAMEWORK_DIR..."

# Remove trailing whitespace
find "$FRAMEWORK_DIR" -name "*.py" -exec sed -i '' 's/[[:space:]]*$//' {} \;

# Add missing final newlines
find "$FRAMEWORK_DIR" -name "*.py" | while read file; do
    if [[ ! -s "$file" || $(tail -c1 "$file" | wc -l) -eq 0 ]]; then
        echo "" >> "$file"
    fi
done

# Check syntax
for file in "$FRAMEWORK_DIR"/*.py; do
    python3 -m py_compile "$file" || echo "❌ Syntax error in $file"
done

# Check line lengths
long_lines=$(find "$FRAMEWORK_DIR" -name "*.py" -exec awk 'length($0) > 80' {} \; | wc -l)
echo "Lines over 80 characters: $long_lines"
```

### Step 7.2: Comprehensive Pylint Error Handling

**🚨 MANDATORY: 10.00/10 PYLINT SCORE REQUIRED**

**Run Pylint Check with Project Configuration:**
```bash
# ALWAYS use venv first, then run from SDK directory  
source venv/bin/activate
cd sdk/python
python -m pylint src/openlit/instrumentation/{framework}/ --rcfile=.pylintrc

# TARGET SCORE: 10.00/10 (not 9.5, must be perfect)
```

**Run Script for Perfect 10.0/10:**
```bash
# Use the script pattern from mem0 success
#!/bin/bash
cd /Users/user/openlit  # Adjust path
source venv/bin/activate
export PYTHONPATH="/Users/user/openlit/sdk/python/src:$PYTHONPATH"
cd sdk/python
python -m pylint src/openlit/instrumentation/{framework}/ --rcfile=.pylintrc
```

**Note**: The project's `.pylintrc` disables many common warnings like `broad-exception-caught`, `too-many-locals`, etc.

### Step 7.2b: Pylint Optimization Patterns

**Common High-Impact Fixes:**

```python
# 1. Lazy logging (prevents W1203: logging-fstring-interpolation)
# ❌ BAD:
logger.debug(f"Failed to extract messages: {e}")

# ✅ GOOD:
logger.debug("Failed to extract messages: %s", e)

# 2. Remove unused imports and variables
# ❌ BAD:
import importlib.metadata
from typing import Dict, Any, Optional, List, Union, Tuple

def wrapper(wrapped, instance, args, kwargs):
    method_name = wrapped.__name__  # Unused variable
    return wrapped(*args, **kwargs)

# ✅ GOOD:
import json
from typing import Dict, Any, Optional, List, Tuple

def wrapper(wrapped, instance, args, kwargs):
    return wrapped(*args, **kwargs)

# 3. Fix exception handling
# ❌ BAD:
try:
    # some code
except Exception as e:  # Unused variable
    pass

# ✅ GOOD:
try:
    # some code
except Exception:
    pass

# 4. Add class methods for R0903: Too few public methods
# ❌ BAD:
class CreateContext:
    def __init__(self):
        self.data = {}

# ✅ GOOD:
class CreateContext:
    """Context for agent creation instrumentation."""
    def __init__(self):
        self.data = {}
    
    def get_context_info(self):
        """Get context information."""
        return self.data
    
    def has_data(self):
        """Check if context has data."""
        return bool(self.data)
```

### Step 7.3: Configure Project Pylint Rules

**Add `import-outside-toplevel` to disabled rules in `sdk/python/.pylintrc`:**

```ini
[MESSAGES CONTROL]
disable=
    # ... existing rules ...
    import-outside-toplevel    # Essential for optional dependencies
```

**Why**: Import-outside-toplevel warnings are triggered by correct optional dependency patterns in OpenLIT.

### Step 7.4: Optimize Dummy Classes for Perfect Score

**Eliminate unnecessary dummy classes to achieve 10.00/10:**

#### **Use TYPE_CHECKING Pattern for Type-Only Classes**
```python
from typing import TYPE_CHECKING

try:
    from framework import MainClass
    if TYPE_CHECKING:
        from framework import TypeOnlyClass1, TypeOnlyClass2
    AVAILABLE = True
except ImportError:
    class MainClass:
        """Dummy class - actually used at runtime"""
        def method(self): return None
    
    if TYPE_CHECKING:
        # Type hints only - don't exist at runtime
        TypeOnlyClass1 = Any
        TypeOnlyClass2 = Any
    AVAILABLE = False

# Use quoted type hints for methods
def process(self, item: "TypeOnlyClass1") -> None:
```

#### **When R0903 is Still Acceptable**
- **Status**: ✅ **ACCEPTABLE** for classes actually used at runtime
- **Example**: TracingProcessor base class that's inherited from

**Target Score**: **10.00/10** is achievable for most instrumentations

### Step 7.5: Common Pylint Issues and Fixes

#### **C0301: Line Too Long**
**Problem**: Lines exceed character limit (135 characters per project pylintrc)

**Fix Pattern**:
```python
# BAD: Long function call
span.set_attribute(SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, handle_not_given(kwargs.get("frequency_penalty"), 0.0))

# GOOD: Split across multiple lines
span.set_attribute(
    SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, 
    handle_not_given(kwargs.get("frequency_penalty"), 0.0)
)

# BAD: Long function call with many parameters
common_span_attributes(scope, SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT, SemanticConvention.GEN_AI_SYSTEM_OPENAI, server_address, server_port, request_model, response_model, environment, application_name, is_stream, tbt, ttft, version)

# GOOD: Split parameters logically
common_span_attributes(
    scope,
    SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT, 
    SemanticConvention.GEN_AI_SYSTEM_OPENAI,
    server_address, server_port, request_model, 
    response_model, environment, application_name, 
    is_stream, tbt, ttft, version
)

# BAD: Long import
from openlit.instrumentation.framework.async_framework import async_general_wrap

# GOOD: Split import
from openlit.instrumentation.framework.async_framework import (
    async_general_wrap
)
```

#### **C0303: Trailing Whitespace**
**Automated Fix**:
```bash
# Remove all trailing whitespace
find src/openlit/instrumentation/{framework} -name "*.py" -exec sed -i '' 's/[[:space:]]*$//' {} \;
```

#### **C0304: Missing Final Newline**
**Automated Fix**:
```bash
# Add missing final newlines
find src/openlit/instrumentation/{framework} -name "*.py" | while read file; do
    if [[ ! -s "$file" || $(tail -c1 "$file" | wc -l) -eq 0 ]]; then
        echo "" >> "$file"
    fi
done
```

#### **C0115: Missing Class Docstring**
**Fix Pattern**:
```python
# BAD: No docstring
class TracingProcessor:
    def force_flush(self): pass

# GOOD: Add descriptive docstring
class TracingProcessor:
    """Dummy TracingProcessor class for when agents is not available"""
    
    def force_flush(self):
        """Dummy force_flush method"""
        pass
```

#### **C0116: Missing Function Docstring**
**Fix Pattern**:
```python
# BAD: No docstring
def _extract_model_info(self, data):
    return data.model

# GOOD: Add descriptive docstring
def _extract_model_info(self, data):
    """Extract model information from span data or agent configuration"""
    return data.model
```

#### **C0321: Multiple Statements on Single Line**
**Fix Pattern**:
```python
# BAD: Multiple statements
def force_flush(self): pass
class Trace: pass

# GOOD: Separate lines with proper formatting
def force_flush(self):
    """Dummy force_flush method"""
    pass

class Trace:
    """Dummy Trace class for when agents is not available"""
    pass
```

#### **C0415: Import Outside Toplevel**
**Context**: These warnings are **ACCEPTABLE** and **REQUIRED** for optional dependencies

**Correct Pattern (DO NOT "FIX")**:
```python
# This is CORRECT for optional dependencies - keep as-is
def _instrument(self, **kwargs):
    try:
        from agents import set_trace_processors  # CORRECT inside function
        set_trace_processors([processor])
    except ImportError:
        pass  # Package not available
```

**Why This is Correct**:
- Prevents import errors when target package isn't installed
- Essential for OpenLIT's optional dependency pattern
- Moving to top level would break instrumentation

**Only Fix When**:
```python
# ONLY move to top level for always-needed imports
import json  # At top of file when always used
from typing import Any, Dict, Optional

def _capture_model_parameters(self, span, data):
    try:
        params = {"model": data.model}
        span.set_attribute("gen_ai.request.parameters", json.dumps(params))
    except Exception:
        pass
```

#### **E0602: Undefined Variable**
**Problem**: Variable used before definition

**Fix Pattern**:
```python
# BAD: Undefined variable
cost = get_chat_model_cost(model, pricing_info, input_tokens, output_tokens)

# GOOD: Import the function
from openlit.__helpers import (
    common_framework_span_attributes,
    handle_exception,
    get_chat_model_cost  # Add missing import
)
```

#### **Syntax Errors: Missing except/finally**
**Problem**: Try block without proper exception handling

**Fix Pattern**:
```python
# BAD: Missing except block
def _extract_token_usage(self, span, data):
    try:
        usage = data.usage
        span.set_attribute("gen_ai.usage.input_tokens", usage.input_tokens)

# GOOD: Add except block
def _extract_token_usage(self, span, data):
    try:
        usage = data.usage
        span.set_attribute("gen_ai.usage.input_tokens", usage.input_tokens)
    except Exception:
        pass  # Ignore errors in token usage extraction
```

#### **R1702: Too Many Nested Blocks**
**Fix Pattern**:
```python
# BAD: Deep nesting
def process_data(self, data):
    if hasattr(data, 'config'):
        if data.config:
            if hasattr(data.config, 'model'):
                if data.config.model:
                    if isinstance(data.config.model, str):
                        return data.config.model

# GOOD: Early returns
def process_data(self, data):
    if not hasattr(data, 'config'):
        return None
    if not data.config:
        return None
    if not hasattr(data.config, 'model'):
        return None
    if not data.config.model:
        return None
    if not isinstance(data.config.model, str):
        return None
    return data.config.model
```

#### **R0903: Too Few Public Methods**
**Context**: **ACCEPTABLE** and **EXPECTED** for dummy/placeholder classes

**Correct Pattern (DO NOT "FIX")**:
```python
# This is CORRECT for dummy classes - keep as-is
class TracingProcessor:
    """Dummy TracingProcessor class for when agents is not available"""
    
    def force_flush(self):
        """Dummy force_flush method"""
        return None
    
    def shutdown(self):
        """Dummy shutdown method"""
        return None
```

**Why This is Correct**:
- Dummy classes by design to prevent import errors
- Only need minimal methods to satisfy interfaces
- Adding unnecessary methods would be incorrect

### Step 7.6: Automated Pylint Fix Script

**Create comprehensive fix script:**
```bash
#!/bin/bash
# fix_pylint_issues.sh
FRAMEWORK_DIR="src/openlit/instrumentation/$1"

echo "🔧 Fixing Pylint issues in $FRAMEWORK_DIR..."

# 1. Remove trailing whitespace
echo "  ✂️  Removing trailing whitespace..."
find "$FRAMEWORK_DIR" -name "*.py" -exec sed -i '' 's/[[:space:]]*$//' {} \;

# 2. Add missing final newlines
echo "  📝 Adding missing final newlines..."
find "$FRAMEWORK_DIR" -name "*.py" | while read file; do
    if [[ ! -s "$file" || $(tail -c1 "$file" | wc -l) -eq 0 ]]; then
        echo "" >> "$file"
    fi
done

# 3. Check syntax errors
echo "  🔍 Checking syntax..."
syntax_errors=0
for file in "$FRAMEWORK_DIR"/*.py; do
    if ! python3 -m py_compile "$file" 2>/dev/null; then
        echo "    ❌ Syntax error in $file"
        syntax_errors=$((syntax_errors + 1))
    fi
done

# 4. Check line lengths and provide guidance
echo "  📏 Checking line lengths..."
long_lines=$(find "$FRAMEWORK_DIR" -name "*.py" -exec awk 'length($0) > 135 { print FILENAME ":" NR ":" length($0) ":" $0 }' {} \;)
if [ -n "$long_lines" ]; then
    echo "    ⚠️  Long lines found (>135 chars):"
    echo "$long_lines" | head -5
    echo "    💡 Split long lines using the patterns in the guide"
fi

# 5. Final validation
echo "  ✅ Running final validation..."
if [ $syntax_errors -eq 0 ]; then
    echo "✅ All syntax errors fixed!"
else
    echo "❌ $syntax_errors syntax errors remaining - manual fix required"
fi

# 6. Run limited pylint check
echo "  🔍 Running key pylint checks..."
pylint --disable=all --enable=syntax-error,undefined-variable,trailing-whitespace,missing-final-newline "$FRAMEWORK_DIR" 2>/dev/null || echo "Some pylint issues remain - check manually"

echo "🎉 Pylint fix script completed!"
echo "💡 Run full pylint check and fix remaining issues manually"
```

### Step 7.7: Manual Fix Checklist

**Before committing, verify:**
- [ ] ✅ All files compile without syntax errors
- [ ] ✅ No trailing whitespace
- [ ] ✅ All files end with newline  
- [ ] ✅ No lines over 135 characters (or split appropriately)
- [ ] ✅ All classes have docstrings
- [ ] ✅ All public methods have docstrings
- [ ] ✅ No undefined variables
- [ ] ✅ Try blocks have except/finally
- [ ] ✅ Imports are at top level when possible
- [ ] ✅ Nested blocks reduced where possible

### Step 7.8: Usage Instructions

**Run the fix script:**
```bash
chmod +x fix_pylint_issues.sh
./fix_pylint_issues.sh openai_agents
./fix_pylint_issues.sh haystack
```

**Final validation:**
```bash
# Check specific framework
pylint src/openlit/instrumentation/openai_agents/

# Quick syntax check
python3 -m py_compile src/openlit/instrumentation/openai_agents/*.py

# Test instrumentation still works
cd sdk/python && PYTHONPATH=src python -c "
import openlit
openlit.init()
print('✅ Instrumentation works after pylint fixes')
"
```

### Step 7.9: Testing After Pylint Fixes

**CRITICAL**: Always test that instrumentation works after fixing pylint issues:

```python
# test_after_pylint_fix.py
import sys
sys.path.insert(0, 'sdk/python/src')

import openlit
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import SimpleSpanProcessor

class TestExporter:
    def __init__(self): 
        self.spans = []
    def export(self, spans): 
        self.spans.extend(spans)
        return 0
    def shutdown(self): 
        pass

def test_instrumentation_after_fixes():
    """Test that instrumentation works after pylint fixes"""
    exporter = TestExporter()
    tracer_provider = TracerProvider()
    tracer_provider.add_span_processor(SimpleSpanProcessor(exporter))
    
    # Initialize OpenLIT
    openlit.init(detailed_tracing=True)
    
    # Run basic test based on framework
    # ... framework-specific test code ...
    
    tracer_provider.force_flush(1000)
    
    assert len(exporter.spans) > 0, "Instrumentation should generate spans"
    print(f"✅ Generated {len(exporter.spans)} spans after pylint fixes")
    
    # Validate span naming convention
    for span in exporter.spans:
        assert ' ' in span.name, f"Span '{span.name}' should follow '{{operation_type}} {{operation_name}}' format"
    
    print("✅ All tests passed - instrumentation working correctly")

if __name__ == "__main__":
    test_instrumentation_after_fixes()
```

## Code Standards & Patterns

### Span Naming Convention

**MANDATORY Format**: `{operation_type} {operation_name}`

```python
# ✅ CORRECT
"agent Triage agent"
"chat gpt-4o"
"retrieve documents"
"generate response"
"workflow multi-agent"

# ❌ INCORRECT  
"Triage agent"           # Missing operation type
"agent_execution"        # Wrong format
"chat_completion_gpt4"   # Wrong format
```

### Semantic Conventions Usage

**Extensive usage required - add to semcov.py when needed:**

```python
# Always use semantic conventions
span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, "framework_name")
span.set_attribute(SemanticConvention.GEN_AI_OPERATION_NAME, operation_name)
span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model)

# Add new conventions when needed
class SemanticConvention:
    GEN_AI_AGENT_DESCRIPTION = "gen_ai.agent.description"
    GEN_AI_WORKFLOW_TYPE = "gen_ai.workflow.type"
```

### Business Intelligence Attributes

**Always capture OpenLIT's competitive advantages:**

```python
# Token usage and cost tracking
span.set_attribute(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens)
span.set_attribute(SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens)
span.set_attribute(SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration)

# Content capture with MIME types
span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, content)
span.set_attribute(SemanticConvention.GEN_AI_CONTENT_COMPLETION, response)
```

## Quality Checklist

### Phase Completion Checklist

**Phase 1 - Competitive Analysis:**
- [ ] Competitor repositories cloned and analyzed
- [ ] Integration patterns documented
- [ ] Competitive gaps identified
- [ ] OpenLIT advantages planned

**Phase 2 - Framework Analysis:**
- [ ] Built-in capabilities explored
- [ ] Method discovery completed
- [ ] Integration pattern chosen
- [ ] Operation mapping defined

**Phase 3 - Implementation Strategy:**
- [ ] Span naming convention defined
- [ ] Semantic conventions planned
- [ ] Test infrastructure designed

**Phase 4 - Implementation:**
- [ ] Test script created first
- [ ] Integration pattern implemented
- [ ] Semantic conventions used extensively
- [ ] Business intelligence captured

**Phase 5 - Testing:**
- [ ] Span hierarchy validated
- [ ] Competitive comparison passed
- [ ] Mock testing completed
- [ ] Error resilience tested

**Phase 6 - Optimization:**
- [ ] Performance overhead measured (<10%)
- [ ] Business intelligence validated
- [ ] Competitive advantages confirmed

**Phase 7 - Cleanup:**
- [ ] Code quality checks passed
- [ ] Syntax errors resolved
- [ ] Line length compliant
- [ ] Final validation completed

### Success Criteria

**Instrumentation is complete when:**
1. 🚨 **CRITICAL: Proper Span Hierarchy**: NO "all root spans" - proper parent-child relationships maintained
2. 🚨 **CRITICAL: Threading Context Fixes**: Handles ThreadPoolExecutor and async context propagation  
3. ✅ **Perfect Pylint Score**: Achieves exactly 10.00/10 (not 9.5, must be perfect)
4. ✅ **Mandatory test.py**: Uses exactly "test.py" filename with venv testing
5. ✅ **Competitive Analysis**: Studied OpenInference, OpenLLMetry, AgentOps, LangFuse
6. ✅ **Target SDK Study**: Cloned and analyzed framework's internal structure
7. ✅ **Semantic Conventions**: Uses SemanticConvention extensively (no hardcoded strings)
8. ✅ **Superior Span Coverage**: Generates more/better spans than competitors
9. ✅ **Naming Convention**: Follows `{operation_type} {operation_name}` format
10. ✅ **4-File Structure**: Uses recommended Pattern 4 when needed for complex frameworks
11. ✅ **Context Caching**: Implements performance optimization patterns with __slots__
12. ✅ **Business Intelligence**: Captures comprehensive cost/token/performance data
13. ✅ **Performance**: Overhead <10% with benchmarking vs competitors
14. ✅ **Error Resilience**: Handles framework version differences gracefully

**⚠️ PRIORITY ORDER**: Span hierarchy (#1) and threading fixes (#2) are MOST CRITICAL - everything else is secondary.

**OpenLIT Competitive Advantages Delivered:**
- 🎯 **Superior Business Intelligence**: Detailed cost tracking, token usage, performance metrics
- 🔗 **Cross-System Integration**: Links to LLM provider spans with proper hierarchy
- 🎨 **Enhanced Observability**: MIME types, structured content capture
- 🛡️ **Error Resilience**: Graceful degradation across framework versions
- 📊 **Comprehensive Coverage**: More spans and attributes than all competitors
- ⚡ **Performance Leadership**: Context caching and optimization patterns
- 🏗️ **Flexible Architecture**: Adapts to different framework patterns (processor, callback, wrapper) 