# Knowledge Graph Ontology

## Last Updated: 2025-02-17
## Stats: 1506 entities, 2140 relations

## Entity Types (top 15)

| Type | Count | Description |
|------|-------|-------------|
| Interface | 483 | API boundaries, traits, protocol interfaces |
| Source | 133 | Source files, modules, crates |
| Component | 126 | Architectural components, subsystems |
| Symbol | 126 | Functions, structs, enums, type definitions |
| Finding | 121 | Analysis findings, observations |
| RepoPath | 90 | File/directory paths in repositories |
| Concept | 86 | Design concepts, architectural patterns |
| Decision | 58 | Design decisions, trade-offs |
| BeadsTask | 54 | Task tracking items |
| CodeArtifact | 46 | Implemented code artifacts |
| TestEvidence | 45 | Test results, coverage data |
| BeadsEpic | 21 | Epic-level task groups |
| Gate | 20 | Quality/review gates |
| Risk | 11 | Identified risks |
| DependencyEdge | 9 | Dependency relationships |
| CodeComponent | 3 | Key code components (AgentPool, etc.) |
| APIEndpoints | 1 | REST API endpoint groups |

## Relation Types (top 15)

| Type | Count | Description |
|------|-------|-------------|
| related_to | 626 | General association |
| documents | 222 | Documentation relationship |
| defines | 165 | Definition relationship |
| derived_from | 137 | Derivation/inheritance |
| defined_in | 126 | Location of definition |
| implements | 122 | Implementation of spec/design |
| depends_on | 77 | Dependency relationship |
| uses | 69 | Usage relationship |
| located_at | 62 | File/path location |
| validated_by | 52 | Test/validation evidence |
| owns | 46 | Ownership relationship |
| documented_by | 43 | Reverse documentation link |
| affects | 34 | Impact relationship |
| implemented_by | 26 | Reverse implementation link |
| belongs_to | 23 | Membership/containment |

## Key Entity Clusters

### A2A Protocol
- `A2A_Protocol_Spec` — Full protocol specification
- `A2A_DataModel` — Core data types (Task, Message, Part, Artifact)
- `A2A_Operations` — RPC operations (sendMessage, getTask, etc.)
- `A2A_ProtocolBindings` — HTTP/gRPC/SSE transport bindings
- `A2A_Security` — Authentication schemes
- `A2A_Extensions` — Extension mechanism
- `A2A_JS_Implementation` — Reference JS implementation analysis
- `A2A_Spec_vs_JS_Gaps` — Gaps between spec and JS code
- `A2A_Crate_Architecture` — Rust crate design
- `A2A_Crate_Implementation_Summary` — Implementation status

### A2A Integration
- `A2A_Goose_Integration_Surface` — Where A2A touches Goose
- `A2A_Goose_Alignment_Assessment` — Alignment analysis
- `A2A_Library_Design_Principles` — Design decisions
- `A2A_Follow_Up_Plan` — Follow-up tasks
- `A2A_Instance_Routes` — REST API for pool instances

### Goose Architecture
- `Goose_Crate_Structure` — Crate layout and dependencies
- `Goose_AgentManager` — Session/agent management
- `Goose_SubAgent_System` — SummonExtension orchestration
- `Goose_ACP_Compat` — ACP protocol support
- `AgentPool` — Parallel agent instance management

### Multi-Protocol
- `Multi_Protocol_Architecture_Design` — A2A + ACP unified design
- `Multi_Instance_Agent_Architecture` — 4-phase multi-instance plan (COMPLETE)

### Design Documents
- `tmp/docs/a2a-protocol-research.mdx`
- `tmp/docs/a2a-crate-architecture.mdx`
- `tmp/docs/a2a-goose-alignment-assessment.mdx`
- `tmp/docs/a2a-vs-acp-deep-dive.mdx`
- `tmp/docs/multi-instance-agent-architecture.mdx`
- `tmp/docs/subagent-a2a-assessment.mdx`

## File Sizes
- `graph.json`: 3,130 KB (full graph)
- `graph.json.gz`: 638 KB (compressed)
- `entities.json`: 2,715 KB (entities only)
- `relations.json`: 291 KB (relations only)
