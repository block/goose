# Evolving Agents Toolkit - Architecture

This document details the architectural design of the Evolving Agents Toolkit (EAT), focusing on its core components, interactions, its unified **MongoDB backend**, and the role of the **Smart Memory ecosystem** in enabling advanced autonomous learning and evolution.

## 1. Introduction & Philosophy

The Evolving Agents Toolkit aims to provide a robust framework for building *ecosystems* of autonomous AI agents. The core philosophy is **agent-centric**: the system itself is managed and orchestrated by specialized agents (like the `SystemAgent`), which leverage tools to interact with underlying services and manage other components. All primary data, including component metadata, embeddings, agent registrations, operational logs, LLM caches, intent plans, and **detailed agent experiences (Smart Memory)**, is now persisted in **MongoDB**.

Key goals of the architecture include:

*   **Autonomy & Evolution:** Enable agents, components, and even system strategies to be created, evaluated, and improved over time, significantly informed by past experiences.
*   **Modularity & Reusability:** Promote component reuse through discovery and adaptation via the `SmartLibrary`.
*   **Interoperability:** Support agents and tools built with different underlying frameworks.
*   **Decoupled Communication:** Facilitate capability-based communication via the `SmartAgentBus`.
*   **Governance & Safety:** Embed safety through `Firmware` and an optional human-in-the-loop review process.
*   **Deep Contextual Understanding:** Provide agents with rich, task-relevant context dynamically constructed from `SmartLibrary`, current task data, and **historical experiences from Smart Memory**.
*   **Orchestration:** Enable complex goal achievement through `SystemAgent`-driven workflow generation and execution.
*   **Cumulative Learning & Self-Improvement:** The **Smart Memory ecosystem** is central to enabling agents, particularly `SystemAgent`, to learn from past workflows, decisions, and outcomes. This learning directly informs better problem-solving, more effective component evolution, and potentially the evolution of the system's own operational strategies.
*   **Unified & Scalable Backend:** Utilize MongoDB for all persistent data.

---
```mermaid
graph TD
    User["User / External System"] -- High-Level Goal --> SA[("SystemAgent\n(Orchestrator, Learner)")];;;agent

    subgraph "Core Infrastructure & Services (MongoDB Backend)"
        direction LR
        SL["Smart Library\n(Components, Versions)"];;;service
        SM["Smart Memory\n(Agent Experiences - `eat_agent_experiences`)"];;;service
        SB["Smart Agent Bus\n(Discovery, Routing - `eat_agent_registry`, `eat_agent_bus_logs`)"];;;service
        LLMS["LLM Service\n(Reasoning, Embeddings, Generation - Cache: `eat_llm_cache`)"];;;service
        FW["Firmware\n(Governance Rules)"];;;service
        MongoDB[("MongoDB Atlas / Server\n(Primary Data Store, Vector Search)")]:::infra
    end
    
    subgraph "Key Agents & Factories"
        direction LR
        MMA[("MemoryManagerAgent\n(Manages Smart Memory via Bus)")];;;agent
        AgentF["Agent Factory"];;;infra
        ToolF["Tool Factory"];;;infra
        ArchZ["ArchitectZero\n(Optional: Solution Design)"];;;agent
        EvoS["EvolutionStrategistAgent\n(Optional: Proactive Evolution Suggestions)"];;;agent
    end

    %% SystemAgent Core Interactions
    SA -- Uses --> ToolsSystem["SystemAgent Tools\n(Search, Create, Evolve, Request, Workflow, IntentReview, ContextBuilder, ExperienceRecorder...)"];;;tool
    SA -- Uses --> LLMS
    SA -- Relies on --> AgentF
    SA -- Relies on --> ToolF

    %% Smart Memory Interactions
    ToolsSystem -- Records/Retrieves Experiences via Bus --> MMA
    MMA -- Manages --> SM
    SM -- Stores/Retrieves --> MongoDB
    MMA -- Uses Internal Tools --> LLMS
    %% For summarization, embedding (if not done by MongoExperienceStoreTool)

    %% SmartLibrary Interactions
    ToolsSystem -- Manages Components --> SL
    SL -- Stores/Retrieves --> MongoDB
    SL -- Uses --> LLMS 
    %% For T_raz, embeddings

    %% AgentBus Interactions
    ToolsSystem -- Interacts via --> SB
    SB -- Stores Registry/Logs --> MongoDB
    MMA -- Registers/Uses --> SB
    ArchZ -- Registers/Uses --> SB
    EvoS -- Registers/Uses --> SB
    Ecosystem["Managed Agents / Tools"] -- Registers/Uses --> SB

    %% Evolution Strategist Interactions (Optional)
    SA -.->|Optional: Requests Evolution Insights| EvoS
    EvoS -- Analyzes --> SL
    EvoS -- Analyzes --> SM
    EvoS -- Analyzes --> ComponentTracker["ComponentExperienceTracker\n(Metrics in AgentBus logs or dedicated store)"]
    %% ComponentExperienceTracker data source

    %% Other Dependencies
    AgentF -- Uses --> Providers["Providers\n(BeeAI, OpenAI, etc)"]
    AgentF -- Uses --> SL
    ToolF -- Uses --> SL
    ToolF -- Uses --> LLMS
    ToolsSystem -- Influenced By --> FW

    %% Optional Design Interaction
    SA -.->|Optional: Requests Design via Bus| ArchZ

    %% Final Result Flow
    SA -- Final Result --> User

    classDef agent fill:#9cf,stroke:#333,stroke-width:2px;
    classDef service fill:#f9f,stroke:#333,stroke-width:2px;
    classDef tool fill:#ccf,stroke:#333,stroke-width:2px;
    classDef infra fill:#eee,stroke:#666,stroke-width:1px,color:#333;
    style SA fill:#69c,stroke:#000,stroke-width:3px,color:#fff;
    style SM fill:#c9f,stroke:#333,stroke-width:2px; %% Highlight Smart Memory
    style MMA fill:#9cf,stroke:#333,stroke-width:2px;
```
*Diagram Key: `agent` = Core EAT Agent, `service` = Core EAT Service, `tool` = SystemAgent's Internal Tools, `infra` = Supporting Infrastructure. Smart Memory is highlighted as a key service.*

---

## 2. Core Components

The toolkit is composed of several key interacting components, all leveraging MongoDB for persistence where applicable.

### 2.1. SystemAgent

The central orchestrator, a `beeai_framework.agents.react.ReActAgent`.
*   **Role:**
    *   Manages the lifecycle of components (agents, tools) by searching the `SmartLibrary`, creating new ones (using `CreateComponentTool`), or evolving existing ones (using `EvolveComponentTool`).
    *   Facilitates communication and task delegation via the `SmartAgentBus` (using `RequestAgentTool`, `DiscoverAgentTool`).
    *   Handles complex, multi-step task execution, often by generating and processing internal workflows (using `GenerateWorkflowTool`, `ProcessWorkflowTool`).
    *   **Crucially, actively utilizes the Smart Memory ecosystem for enhanced decision-making:**
        *   Uses `ContextBuilderTool` before significant planning or component selection to gather relevant past experiences and message summaries from the `MemoryManagerAgent`. This provides deep, historical context.
        *   Uses `ExperienceRecorderTool` after completing tasks or workflows to record significant outcomes, decisions, and learnings into Smart Memory, fostering continuous improvement for itself and the system.
    *   Manages optional human-in-the-loop review processes via intent review tools.
*   **Key Tools (Expanded):**
    *   **SmartLibrary Tools:** `SearchComponentTool`, `CreateComponentTool`, `EvolveComponentTool`.
    *   **SmartContext & Memory Tools:** `TaskContextTool` (for T_raz generation for search queries), `ContextBuilderTool` (constructs rich context using Smart Memory and SmartLibrary), `ExperienceRecorderTool` (records outcomes to Smart Memory).
    *   **AgentBus Tools:** `RegisterAgentTool`, `RequestAgentTool`, `DiscoverAgentTool`.
    *   **Workflow Tools:** `GenerateWorkflowTool`, `ProcessWorkflowTool`.
    *   **Intent Review Tools (Optional):** `WorkflowDesignReviewTool`, `ComponentSelectionReviewTool`, `ApprovePlanTool`.

### 2.2. ArchitectZero Agent

A specialized `ReActAgent` for designing solutions, typically invoked by `SystemAgent` via the `SmartAgentBus`.
*   **Role:** Analyzes complex requirements, queries `SmartLibrary` and potentially `SmartMemory` (if equipped with tools or via SystemAgent proxy) for existing patterns or components, and designs multi-component solutions.
*   **Output:** A structured solution design (e.g., JSON) that `SystemAgent` can use as a blueprint for workflow generation or component creation.

### 2.3 Smart Memory Ecosystem

This is a critical addition for enabling advanced learning and autonomous evolution.
*   **`MemoryManagerAgent`**:
    *   **Role**: A `ReActAgent` acting as the central orchestrator for the storage and retrieval of long-term memories (experiences). Registered on the `SmartAgentBus` (e.g., `memory_manager_agent_default_id`). It exposes a general `process_task` capability, allowing other agents (primarily `SystemAgent` via its tools) to send natural language requests for memory operations (e.g., "store this experience: {...}", "find experiences related to: 'invoice processing errors'").
    *   **Internal Tools**:
        *   `MongoExperienceStoreTool`: Handles CRUD operations for experiences in the `eat_agent_experiences` MongoDB collection, including generating embeddings for searchable text fields within an experience.
        *   `SemanticExperienceSearchTool`: Performs semantic (vector) searches over stored experiences in `eat_agent_experiences` based on natural language queries.
        *   `MessageSummarizationTool`: Uses an LLM to summarize message histories relevant to a specific goal, often requested by `ContextBuilderTool`.
*   **`ContextBuilderTool` (Tool for `SystemAgent`)**:
    *   **Purpose**: Dynamically constructs an optimized `SmartContext` instance for `SystemAgent` to use for a specific sub-task.
    *   **Functionality**: Invoked by `SystemAgent`. It uses `RequestAgentTool` to query the `MemoryManagerAgent` (to retrieve relevant past experiences and message summaries) and also directly queries `SmartLibrary` for relevant components. It assembles this information into a focused `SmartContext`.
*   **`ExperienceRecorderTool` (Tool for `SystemAgent`)**:
    *   **Purpose**: Facilitates the recording of completed tasks, sub-tasks, or entire workflows as structured experiences.
    *   **Functionality**: Invoked by `SystemAgent`. It structures task details (goal, components used, inputs, decisions, outputs, outcome, reasoning snippets) and uses `RequestAgentTool` to send this structured data to the `MemoryManagerAgent` for persistence in Smart Memory.
*   **MongoDB Collection: `eat_agent_experiences`**:
    *   **Purpose**: Stores structured records of agent experiences.
    *   **Key Fields**: Includes `experience_id`, `primary_goal_description` (embedded), `sub_task_description` (embedded), `input_context_summary` (embedded), `components_used`, `key_decisions_made`, `final_outcome`, `output_summary` (embedded), `feedback_signals`, `timestamp`, and generated `embeddings` for its searchable text fields. (A detailed schema is in `eat_agent_experiences_schema.md`).
    *   **Vector Search Index:** Requires an Atlas Vector Search index (e.g., `vector_index_experiences_default`) on its embedding fields.

### 2.4. Smart Library (MongoDB Backend)

Persistent storage and discovery for reusable components (agents, tools, firmware).
*   **MongoDB Collection:** `eat_components` (or configurable name).
*   **Stores:** Each component as a MongoDB document including its `id`, `name`, `record_type`, `domain`, `description`, `code_snippet`, `version`, `tags`, `metadata` (like `applicability_text`), and crucially, its `content_embedding` (E_orig) and `applicability_embedding` (E_raz).
*   **Discovery (Dual Embedding on MongoDB):**
    *   Uses **MongoDB Atlas Vector Search** (or equivalent) on the `content_embedding` and `applicability_embedding` fields within the `eat_components` collection.
    *   Supports **Task-Aware Semantic Search**: Primarily queries against `applicability_embedding` using task context, then refines using `content_embedding` similarity.
*   **Versioning & Evolution:** Tracks component versions and parentage within MongoDB documents.
*   **Interface:** Methods like `create_record`, `find_record_by_id`, `semantic_search`, `evolve_record` now perform CRUD and search operations against MongoDB.
*   **Indexing Pipeline:** When records are created/updated:
    *   `T_orig` (functional text from `_get_record_vector_text`) is embedded to get `content_embedding`.
    *   `T_raz` (applicability text from `generate_applicability_text` via `LLMService`) is generated and embedded to get `applicability_embedding`.
    *   Both embeddings and `T_raz` (in metadata) are stored directly in the component's MongoDB document.

```mermaid
graph TD
    SLI["SmartLibrary Interface"] -->|Manages| Records["Component Records (MongoDB Documents)\n(id, name, ..., content_embedding, applicability_embedding, metadata.applicability_text)"]
    Records -- Stored in --> MongoDBLib["MongoDB (eat_components collection)"]:::infra
    MongoDBLib -- Vector Indexes on --> VIdxContent["Content Embedding (E_orig)"]
    MongoDBLib -- Vector Indexes on --> VIdxApp["Applicability Embedding (E_raz)"]

    SLI -->|Uses| LLMSLib["LLM Service (Embeddings & T_raz Generation)"]

    subgraph "Storage & Search (MongoDB)"
        direction LR
        Records -- CRUD Ops --> MongoDBLib
        VIdxContent -- Used by --> SemanticSearch["SmartLibrary.semantic_search()\n(MongoDB $vectorSearch)"]
        VIdxApp -- Used by --> SemanticSearch
    end

    LLMSLib -- Generates E_orig, E_raz --> Records
    LLMSLib -- Generates T_raz --> Records[metadata.applicability_text]

    classDef infra fill:#eee,stroke:#666,stroke-width:1px,color:#333;
```
*Diagram Note: `MongoDBLib` and `LLMSLib` are used to avoid name clashes with the main diagram's `MongoDB` and `LLMS` if rendered separately.*

### 2.5. Smart Agent Bus (MongoDB Backend)

Manages inter-agent communication and capability discovery.
*   **Agent Registry (MongoDB Collection):** `eat_agent_registry`. Stores agent registration documents (id, name, capabilities, status, etc.). Key infrastructure agents like `MemoryManagerAgent` are registered here.
    *   Optionally, capability descriptions can be embedded (`capability_description_embedding`) and stored here for semantic discovery of capabilities using MongoDB Vector Search.
*   **Execution Logs (MongoDB Collection):** `eat_agent_bus_logs`. Stores logs of System and Data Bus operations, providing valuable data for `ComponentExperienceTracker` and auditing.
*   **Role:** Central nervous system for capability-based requests (Data Bus) and agent/service registration/discovery (System Bus).
*   **Discovery:**
    *   Finds registered agents based on `capability_id` (metadata query on `eat_agent_registry`).
    *   For `task_description`-based discovery, it can query agent descriptions (potentially with vector search on `description_embedding` in `eat_agent_registry`) or leverage `SmartLibrary` to find agent *components* suitable for the task.
*   **Resilience:** Circuit breakers (currently file-based, future MongoDB) help manage agent availability.
*   **Interface:** Methods like `register_agent`, `discover_agents`, `request_capability`, etc., now interact with MongoDB.

```mermaid
graph TD
    SABI["AgentBus Interface"] -->|Manages| Reg["Agent Registry (MongoDB 'eat_agent_registry')"]
    SABI -->|Logs to| ExecLogs["Execution Logs (MongoDB 'eat_agent_bus_logs')"]
    SABI -->|Uses for Discovery| LLMSBus["LLM Service (for embedding query if vector searching descriptions)"]
    SABI -->|Monitors| CB["Circuit Breakers (File-based)"]

    Reg -- Optional Vector Index on --> AgentDescEmbeds["Agent Description Embeddings"]
    Reg -- Optional Vector Index on --> CapEmbeds["Capability Description Embeddings"]

    subgraph "Data Bus Communication (request_capability)"
        direction LR
        Requester --> SABI
        SABI -->|"Finds Provider (Queries Reg)"| Reg 
        SABI -->|"Routes Request"| Provider["Agent/Tool Instance"]
        Provider -->|"Returns Result"| SABI
        SABI -->|"Returns Result"| Requester
    end
```
*Diagram Note: `LLMSBus` used for clarity. Discovery uses MongoDB queries (metadata or vector).*

### 2.6. Smart Context

Data structure for passing task-relevant information between components, especially to agents.
*   **Role:** Carries current task data, user input, intermediate results from previous steps, and a description of the current task (`current_task`).
*   **Enhancement via `ContextBuilderTool`**: It is **dynamically enriched** by the `ContextBuilderTool`. This tool populates the `SmartContext` with:
    *   Summaries of relevant past experiences and message histories (retrieved via the `MemoryManagerAgent` from `SmartMemory`).
    *   Information about relevant existing components (retrieved from `SmartLibrary`).
    This provides agents like `SystemAgent` with deeper, historically-informed context for their current tasks.
*   **Interaction with `SmartLibrary`:** The `current_task` description it holds (often enhanced) is used by `SmartLibrary.semantic_search` to leverage the dual embeddings (E_raz) for more accurate, task-aware component retrieval.

### 2.7. EvolutionStrategistAgent (Optional)

*   **Role:** A specialized agent (potentially a `ReActAgent`) designed to proactively analyze system performance, component experiences, and library structure to identify and suggest opportunities for evolution. It acts as a higher-level "improvement engine."
*   **Data Sources for Analysis:**
    *   `ComponentExperienceTracker`: For quantitative performance metrics (success/failure rates, execution times, error frequencies) derived from `SmartAgentBus` logs or a dedicated metrics store.
    *   `SmartMemory` (via `MemoryManagerAgent`): To analyze patterns in successful/failed experiences, understanding *why* components perform as they do in specific contexts. This provides qualitative insights.
    *   `SmartLibrary`: To understand the current inventory of components, their versions, and identify gaps or redundancies.
    *   A/B Test Results (if applicable, from `eat_ab_test_results` collection): To compare versions of components.
    *   `LibraryAuditorAgent` outputs: Can consume reports about unused, orphaned, or low-performing components.
*   **Output:** Evolution proposals or directives. These could be:
    *   Suggestions to `SystemAgent` (e.g., "Evolve component X to better handle task Y, as it has failed N times. Consider incorporating logic from component Z which succeeded on similar tasks.").
    *   Direct initiation of evolution tasks via `EvolveComponentTool` if granted sufficient autonomy.
    *   Proposals for creating entirely new components to fill identified capability gaps.

### 2.8. Providers & Agent Factory

*   **`Providers` (e.g., `BeeAIProvider`, `OpenAIAgentsProvider`):** Abstract the specifics of interacting with different underlying AI agent frameworks. They handle the creation and execution of agents built with those frameworks.
*   **`AgentFactory`:** Uses the `ProviderRegistry` to select the appropriate `FrameworkProvider` based on a component's specified framework (e.g., "beeai", "openai-agents"). It then delegates agent creation to that provider. It can also instantiate tools directly if they are simple Python classes.

### 2.9. Dependency Container

*   **Role:** A simple Inversion of Control (IoC) container (`DependencyContainer`) manages the instantiation and sharing of core singleton services like `MongoDBClient`, `LLMService`, `SmartLibrary`, `SmartAgentBus`, and key agents like `SystemAgent`, `MemoryManagerAgent`.
*   **Benefits:** Decouples components, simplifies initialization, and makes dependencies explicit. Components can request their dependencies from the container rather than instantiating them directly.

### 2.10. LLM Service (MongoDB Cache)

*   **Role:** Provides a unified interface for interacting with Large Language Models (LLMs) for tasks like text generation, code generation, and text embedding.
*   **Frameworks:** Utilizes `LiteLLM` (via `beeai-framework`) for broad compatibility with various LLM providers (OpenAI, Anthropic, Ollama, etc.).
*   **Caching (`LLMCache` on MongoDB):**
    *   LLM completions and embedding results are cached to reduce redundant API calls, save costs, and speed up repeated operations.
    *   The cache (`eat_llm_cache` collection in MongoDB) uses Time-To-Live (TTL) indexes for automatic expiration of old entries.

### 2.11. Firmware

*   **Role:** The `Firmware` class provides a mechanism to inject standardized governance rules, ethical guidelines, and operational constraints into agents and tools, particularly during their creation or evolution.
*   **Functionality:** It generates a base prompt incorporating general rules and can include domain-specific rules (e.g., for "medical" or "finance"). This prompt is typically prepended to the main instruction set given to LLMs when generating or modifying component code or behavior.
*   **Persistence:** Firmware rules themselves are typically code-defined within the `Firmware` class, not dynamically stored as individual `SmartLibrary` components, though specific firmware *configurations* or *overrides* could potentially be.

### 2.12. Adapters

*   **Role:** `Adapters` (e.g., `OpenAIToolAdapter`, `OpenAIGuardrailsAdapter`) serve as bridges between EAT's internal component structures/interfaces and the specific requirements of external AI agent SDKs or frameworks (like OpenAI Agents SDK).
*   **Functionality:**
    *   `OpenAIToolAdapter`: Converts EAT-compatible tools (e.g., BeeAI tools, dynamic tools) into a format usable by OpenAI Agents.
    *   `OpenAIGuardrailsAdapter`: Converts EAT's `Firmware` concepts into input/output guardrails compatible with the OpenAI Agents SDK.
*   **Importance:** They enable interoperability and allow EAT to leverage components and functionalities from various ecosystems.

### 2.13. Intent Review System (MongoDB Backend)

*   **Role:** Provides an optional human-in-the-loop (HITL) mechanism for reviewing and approving actions قبل execution, enhancing safety and control.
*   **`IntentPlan` Storage (MongoDB Collection):** `eat_intent_plans`.
    *   When intent review is enabled for the "intents" level, `ProcessWorkflowTool` generates `IntentPlan` objects (detailing each step, its parameters, and justification).
    *   These `IntentPlan` objects are serialized and stored as documents in this MongoDB collection.
*   **Review Process:**
    *   The `IntentReviewAgent` (or a human user directly) uses tools like `ApprovePlanTool`.
    *   `ApprovePlanTool` loads the `IntentPlan` from MongoDB by its `plan_id`, facilitates the review, and then updates the plan's status (e.g., "APPROVED", "REJECTED") and any reviewer comments directly in its MongoDB document.
*   **Other Review Tools:** `WorkflowDesignReviewTool` and `ComponentSelectionReviewTool` allow for earlier-stage reviews, though their outputs might not always be persisted as formal `IntentPlan` objects unless they feed into one.

## 3. Key Architectural Patterns & Flows

### 3.1. Agent Communication (via Agent Bus)

The `SmartAgentBus` is the central nervous system for the EAT ecosystem, facilitating both system-level management and data-level interactions between components. It operates as a dual-bus system, with distinct functionalities for managing the agent/tool registry and for routing capability requests. Both aspects now leverage MongoDB for persistence.

**A. System Bus Functionality (Management & Discovery):**

*   **Registration:**
    *   Agents and tools (components) are registered with the `SmartAgentBus` using tools like `RegisterAgentTool` (typically invoked by `SystemAgent` or during system initialization from `SmartLibrary`).
    *   Registration involves providing metadata about the component, including its ID, name, type (AGENT/TOOL), description, and a list of capabilities it offers.
    *   This registration data is stored in the `eat_agent_registry` MongoDB collection. Each entry includes the component's metadata and, importantly, an embedding of its description (`description_embedding`) if semantic discovery is enabled.
*   **Discovery:**
    *   Agents (like `SystemAgent`) can discover other components based on various criteria:
        *   **By Capability ID:** Requesting all agents that offer a specific `capability_id`. The bus queries the `eat_agent_registry` for matching capability entries.
        *   **By Task Description (Semantic Discovery):** An agent can provide a natural language `task_description`. The `SmartAgentBus` (or a tool like `DiscoverAgentTool`) can:
            1.  Embed this `task_description`.
            2.  Perform a vector search against the `description_embedding` field in the `eat_agent_registry` collection to find agents whose overall purpose matches the task.
            3.  Alternatively, it can leverage `SmartLibrary.semantic_search` to find *component definitions* (which might be more fine-grained) suitable for the task, and then check if those components are registered and active on the bus.
    *   Discovery results include information about the found agents, their status (e.g., active, circuit-broken), and potentially relevance scores.
*   **Status Management:** The bus can track the health and status of registered agents (e.g., using circuit breakers, though the state for these is currently file-based with plans to move to MongoDB).

**B. Data Bus Functionality (Capability Request & Execution):**

*   **Request Routing:**
    *   When an agent (Requester) needs a specific capability performed (e.g., `SystemAgent` using `RequestAgentTool`), it sends a request to the `SmartAgentBus`.
    *   The request typically includes the `capability_id` and the `content` (input data/parameters) for that capability.
    *   The `SmartAgentBus` uses its registry (`eat_agent_registry`) to find one or more active and healthy agents (Providers) that offer the requested `capability_id` and meet any other criteria (like minimum confidence).
    *   If multiple providers are found, the bus might employ a selection strategy (e.g., round-robin, load-based, highest confidence – currently simpler, often takes the first suitable).
*   **Execution:**
    *   The bus routes the request `content` to the selected Provider agent/tool instance.
    *   The Provider executes the capability using the provided `content`.
    *   The result of the execution is returned to the `SmartAgentBus`.
*   **Response Delivery:** The `SmartAgentBus` relays the result back to the original Requester.
*   **Logging:** All significant Data Bus operations (requests, which agent handled it, success/failure, duration) are logged to the `eat_agent_bus_logs` MongoDB collection. This provides an audit trail and data for `ComponentExperienceTracker`.

**C. Key Characteristics:**

*   **Decoupling:** Agents do not need direct references to each other. They request capabilities, and the bus handles routing. This promotes modularity and makes it easier to add, remove, or update components.
*   **Dynamic Discovery:** The system can adapt to new agents becoming available or existing ones changing their capabilities.
*   **Centralized Management:** The bus provides a central point for monitoring agent interactions and managing the ecosystem.
*   **Resilience (via Circuit Breakers):** The bus can temporarily prevent requests to agents that are consistently failing, improving overall system stability.
*   **MongoDB Backend:** Storing the registry and logs in MongoDB enhances scalability, persistence, and the ability to perform complex queries over historical interactions.

This dual-bus architecture, backed by MongoDB, is fundamental to EAT's ability to orchestrate a dynamic and evolving ecosystem of agents and tools.

### 3.2. Task-Aware Context Retrieval (Dual Embedding on MongoDB)

The core logic for how `SmartLibrary` uses dual embeddings remains, but the `task_context` itself is now often significantly richer due to pre-processing by `ContextBuilderTool` which queries `SmartMemory`.

1.  **Query Formulation:** `SystemAgent` (or another agent) has a `query` (what it's looking for) and a `task_description` (the current task it's trying to solve).
2.  **Context Enrichment (via `ContextBuilderTool`):** Often, before searching the library, `SystemAgent` will use `ContextBuilderTool`. This tool queries `SmartMemory` (via `MemoryManagerAgent`) for relevant past experiences and message summaries related to the `task_description`. It also queries `SmartLibrary` for an initial set of relevant components. The output of `ContextBuilderTool` provides an *enriched task context*.
3.  **Embedding for Search:** `LLMService` embeds the original `query` (for E_orig) and the (potentially enriched) `task_description` (for E_raz).
4.  **Applicability Search (MongoDB `$vectorSearch`):** `SmartLibrary.semantic_search` uses the `task_description` embedding (E_raz_query) to query the `applicability_embedding` field in the `eat_components` MongoDB collection. This prioritizes components whose purpose aligns with the current task.
5.  **Refinement & Scoring:**
    *   `SmartLibrary` retrieves candidate component documents from MongoDB, which include their `content_embedding` (E_orig).
    *   It calculates the similarity between the `query` embedding (E_orig_query) and each candidate's `content_embedding`.
    *   A `final_score` is computed by combining the task relevance score (from applicability search) and the content similarity score, often weighted (e.g., `task_weight`). Usage history can also provide a small boost.
6.  **Result:** `SmartLibrary` returns a list of the most relevant component documents, ranked by their `final_score`.

```mermaid
sequenceDiagram
    participant Agent as Requesting Agent (e.g., SystemAgent)
    participant CtxBuilder as ContextBuilderTool
    participant MMA as MemoryManagerAgent
    participant SL as SmartLibrary (MongoDB backed)
    participant MongoDB as MongoDB (eat_components, eat_agent_experiences)
    participant LLM_Emb as Embedding Service

    Agent->>Agent: Formulate Initial Task Query / Description
    Agent->>CtxBuilder: BuildContext(Task Description)
    CtxBuilder->>MMA: RequestRelevantExperiences(Task Description via Bus)
    MMA->>MongoDB: SemanticSearch on 'eat_agent_experiences'
    MongoDB-->>MMA: Past Relevant Experiences
    MMA-->>CtxBuilder: Experiences
    CtxBuilder->>SL: StandardSearchForComponents(Task Description)
    SL->>MongoDB: SemanticSearch on 'eat_components' (can use T_orig and T_raz)
    MongoDB-->>SL: Relevant Library Components
    SL-->>CtxBuilder: Library Components
    CtxBuilder-->>Agent: Enriched SmartContext (contains experiences, components, original task)
    
    Note over Agent: Agent now has richer context for its next action.
    Agent->>Agent: Refine Query & Task Context (using Enriched SmartContext results)
    Agent->>LLM_Emb: Embed Query (E_orig_query), Embed Task Context (E_raz_query)
    LLM_Emb-->>Agent: Query Embeddings

    Agent->>SL: semantic_search(Refined Query, Refined Task Context, ...)
    SL->>MongoDB: $vectorSearch on 'applicability_embedding' with E_raz_query
    MongoDB-->>SL: Top-K Candidate Component Documents
    SL->>SL: Re-rank/Score (combining task and content similarity)
    SL-->>Agent: Final Relevant Component Documents & Scores
```

### 3.3. Agent Learning and Context Enrichment Flow (with Smart Memory)

This is a **central flow** enabled by the Smart Memory ecosystem:

1.  **Task Initiation:** `SystemAgent` receives a high-level goal.
2.  **Context Building (Proactive):** `SystemAgent` uses `ContextBuilderTool`.
    *   `ContextBuilderTool` sends a request to `MemoryManagerAgent` (via `SmartAgentBus`) with the current sub-task description.
    *   `MemoryManagerAgent` uses `SemanticExperienceSearchTool` to find relevant past experiences from `eat_agent_experiences`.
    *   `MemoryManagerAgent` uses `MessageSummarizationTool` to summarize relevant recent message history (if provided to `ContextBuilderTool`).
    *   `ContextBuilderTool` also queries `SmartLibrary` (MongoDB `eat_components`) for potentially relevant existing components.
    *   `ContextBuilderTool` returns a structured dataset containing these findings to `SystemAgent`.
3.  **Informed Planning & Action:** `SystemAgent` incorporates this rich context (past successes/failures, relevant components, message summaries) into its planning:
    *   More accurate component selection (via `SearchComponentTool` now using better task context, or by directly choosing from `ContextBuilderTool`'s suggestions).
    *   Better workflow design (via `GenerateWorkflowTool`).
    *   More targeted parameters for tool/agent execution.
4.  **Task Execution:** `SystemAgent` orchestrates the execution of the planned steps.
5.  **Experience Recording:** After a significant sub-task or the overall task is completed (or fails), `SystemAgent` uses `ExperienceRecorderTool`.
    *   `ExperienceRecorderTool` structures the key details of the just-completed experience (goal, input summary, components used, decisions, output summary, outcome, reasoning snippets).
    *   It sends this structured experience to `MemoryManagerAgent` (via `SmartAgentBus`).
    *   `MemoryManagerAgent` uses `MongoExperienceStoreTool` to save the experience (including generating its embeddings) into the `eat_agent_experiences` collection in MongoDB.
6.  **Learning Loop Closure:** The newly recorded experience is now available for future `ContextBuilderTool` queries, allowing the system to learn and improve over time.

### 3.4. Informed Component Evolution

Smart Memory plays a vital role in making component evolution more targeted and effective:

1.  **Identification of Need:**
    *   `SystemAgent` might identify a need for evolution based on repeated failures for a specific task type (retrieved from Smart Memory).
    *   Or, `EvolutionStrategistAgent` might analyze `eat_agent_experiences` and `ComponentExperienceTracker` data to proactively suggest evolutions.
2.  **Contextual Evolution Prompting:** When `SystemAgent` decides to evolve a component using `EvolveComponentTool`:
    *   It first uses `ContextBuilderTool` to gather context related to the component-to-be-evolved and the problem it's failing to solve (or the new capability needed).
    *   This context (e.g., "Component X failed 3 times on task Y when input was Z, resulting in error W. Component A handles similar inputs P successfully.") is used to formulate a much more specific and informed "changes" description for `EvolveComponentTool`.
3.  **LLM-Driven Evolution:** `EvolveComponentTool` uses the LLM to generate new code based on the parent component's code and this rich, contextual "changes" description.
4.  **New Version in Library:** The evolved component is saved as a new version in `SmartLibrary`.
5.  **Future Use:** This new, contextually-evolved component is now available for future tasks and will be discoverable via `SmartLibrary` searches, further improving system performance.

### 3.5. Workflow Generation & Execution
The `SystemAgent` uses `GenerateWorkflowTool` to create a YAML definition of steps based on a task or a design from `ArchitectZero`. This generation can be informed by context retrieved via `ContextBuilderTool`. The `ProcessWorkflowTool` then parses this YAML, substitutes parameters, and either:
*   Converts it into an `IntentPlan` (if intent review is enabled for 'intents'), which is saved to MongoDB (`eat_intent_plans`) for review.
*   Returns a list of processed steps for direct execution if intent review is not active for that stage.

### 3.6. Dependency Injection & Initialization
The `DependencyContainer` is used at application startup to instantiate and wire together core services (`MongoDBClient`, `LLMService`, `SmartLibrary`, `SmartAgentBus`, `Firmware`) and key agents (`SystemAgent`, `ArchitectZero`, `MemoryManagerAgent`, `IntentReviewAgent`). This promotes loose coupling and centralized management of shared resources. Core components like `SmartLibrary` and `SmartAgentBus` have `initialize()` methods that might perform further setup, like loading data from MongoDB or syncing registries, once all their dependencies are available.

### 3.7. Intent Review / Human-in-the-Loop Flow (MongoDB Backend)

1.  **Design Review (Optional):** `SystemAgent` can use `WorkflowDesignReviewTool` to get human/AI feedback on a high-level solution design (e.g., from `ArchitectZero`) before detailed planning.
2.  **Component Selection Review (Optional):** `SystemAgent` can use `ComponentSelectionReviewTool` after a `SearchComponentTool` run to get human/AI validation of the chosen components.
3.  **Intent Plan Generation & Persistence:** If intent review for the 'intents' level is active, `ProcessWorkflowTool` (used by `SystemAgent`) generates a detailed `IntentPlan` object. This object, containing each step, its parameters, and justification, is serialized and saved as a document in the `eat_intent_plans` MongoDB collection. The `plan_id` is returned.
4.  **Intent Plan Review (Core):** `SystemAgent` (or a human operator) then uses `ApprovePlanTool`, providing the `plan_id`. `ApprovePlanTool` loads the `IntentPlan` from MongoDB, facilitates interactive or AI-assisted review, and updates the plan's status (e.g., to `APPROVED` or `REJECTED`) and reviewer comments directly in its MongoDB document.
5.  **Execution Post-Approval:** If the `IntentPlan` is `APPROVED`, `SystemAgent` would typically load this approved plan (or its constituent intents) from MongoDB to guide the safe execution of the task.

```mermaid
sequenceDiagram
    participant SysA as SystemAgent
    participant PWT as ProcessWorkflowTool
    participant MongoDBReview as MongoDB (eat_intent_plans)
    participant Reviewer as IntentReviewAgent / Human
    participant APT as ApprovePlanTool

    Note over SysA: Task requires workflow
    SysA->>PWT: ProcessWorkflow(workflowYAML, objective, ...)
    Note over PWT: Intent Review Enabled for 'intents'
    PWT->>PWT: Parse YAML, Substitute Params
    PWT->>PWT: Validate Steps & Generate IntentPlan Object
    PWT->>MongoDBReview: Save/Upsert IntentPlan Object (as document)
    MongoDBReview-->>PWT: Confirm Save (e.g., plan_id)
    PWT-->>SysA: Return plan_id (and status: intent_plan_created)

    Note over SysA: Review Required for plan_id
    SysA->>Reviewer: Request Intent Plan Review (Pass plan_id)
    Reviewer->>APT: Use ApprovePlanTool (Input: plan_id)
    APT->>MongoDBReview: Load IntentPlan document by plan_id
    MongoDBReview-->>APT: Return IntentPlan document
    Note over APT: Presents Plan to Reviewer for decision
    APT->>Reviewer: Get Approval/Rejection Decision & Comments
    Reviewer-->>SysA: Return Decision (Approved/Rejected, comments)

    alt Plan Approved
        APT->>MongoDBReview: Update IntentPlan document status to APPROVED, add comments
        SysA->>MongoDBReview: (Later) Load Approved Intents from Plan for execution
        SysA-->>Caller: Final Result
    else Plan Rejected
        APT->>MongoDBReview: Update IntentPlan document status to REJECTED, add reason
        SysA->>SysA: Halt Execution / Report Rejection
        SysA-->>Caller: Report Rejection / Failure
    end
```

## 4. Multi-Framework Integration
EAT is designed for interoperability. The `ProviderRegistry` and `FrameworkProvider` abstract base class allow integration with various agent frameworks (e.g., BeeAI, OpenAI Agents SDK). `AgentFactory` uses this registry to instantiate agents based on the framework specified in their `SmartLibrary` record. Adapters (like `OpenAIToolAdapter`) help bridge EAT's internal tool formats with those expected by specific SDKs. This modular design allows the EAT ecosystem to leverage agents and tools built with different technologies.

## 5. Governance and Safety
Safety and governance are integrated through:
*   **`Firmware`:** Injects common ethical and operational rules into components during their creation or evolution, guiding LLM behavior.
*   **Intent Review System:** Provides optional human-in-the-loop checkpoints at various stages (design, component selection, intent execution) to validate system plans and actions before they proceed. The persistence of `IntentPlan`s in MongoDB aids auditability.
*   **`SmartAgentBus` Circuit Breakers:** Help prevent cascading failures by temporarily disabling agents that repeatedly fail or timeout.
*   **Smart Memory:** While primarily for learning, recorded experiences (especially failures and their contexts) can be analyzed to identify systemic risks or unsafe component behaviors, contributing to long-term safety improvements.

This enhanced architecture, with Smart Memory at its core, provides a more powerful foundation for EAT to achieve its goal of building adaptive, learning, and autonomously evolving AI agent systems. The ability to learn from rich, contextualized past experiences allows the system to make more intelligent decisions about current tasks and future evolutions.