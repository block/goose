---
title: Knowledge Graph Ontology
type: ontology
entity_count: 1493
relation_count: 2120
exported_at: '2026-02-17T00:12:00Z'
---

# Knowledge Graph Ontology

## Schema

### Entity Types

| Type | Count | Examples |
|------|-------|----------|
| Interface | 483 | `Interface: subagent tool schema`, `Interface: SessionType::SubAgent`, `Interface: parse_recipe_content` (+480 more) |
| Source | 133 | `Source: bd CLI snapshots 2026-02-06T15:48:30.383Z`, `Source: bd CLI evidence 2026-02-06T16:48:00`, `Source: rg acp 2026-02-06T20:33:00` (+130 more) |
| Symbol | 126 | `Symbol:RepetitionInspector@crates/goose/src/tool_monitor.rs:34`, `Symbol:InspectionResult@crates/goose/src/tool_inspection.rs:12`, `Symbol:InspectionAction@crates/goose/src/tool_inspection.rs:23` (+123 more) |
| Component | 123 | `Crate: goose`, `Crate: goose-acp`, `Crate: goose-bench` (+120 more) |
| Finding | 119 | `Finding: Beads DB missing initially in goose3`, `Finding: Beads DB initialized in goose3`, `Finding: RPI gate dependency chain created for goose3 epic` (+116 more) |
| RepoPath | 90 | `Repo: /home/jmercier/codes/goose3`, `RepoPath: documentation/beads.md`, `RepoPath: README.md` (+87 more) |
| Concept | 86 | `Concept: GooseAgent`, `Concept: Subagent`, `Concept: ACP` (+83 more) |
| BeadsTask | 54 | `BeadsTask: goose3-61d`, `BeadsTask: goose3-vr0`, `BeadsTask: goose3-mf0` (+51 more) |
| Decision | 52 | `Decision: require --agent when --mode is used`, `Decision: goose session --mode requires --agent`, `Decision: Model 'Verify blocks Close' as process constraint (avoid dependency cycle)` (+49 more) |
| CodeArtifact | 46 | `CodeArtifact:file:crates/goose/src/lib.rs`, `CodeArtifact:file:crates/goose-cli/src/main.rs`, `CodeArtifact:file:crates/goose-server/src/main.rs` (+43 more) |
| TestEvidence | 45 | `TestEvidence: bd ready/blocked snapshot 2026-02-06`, `TestEvidence: bd ready/blocked after wiring 2026-02-06T15:48:30.383Z`, `TestEvidence: Beads ready/blocked snapshot 2026-02-06T16:48:00` (+42 more) |
| BeadsEpic | 21 | `BeadsEpic: goose3-dcq`, `BeadsEpic: goose3-daa`, `BeadsEpic: goose3-hgx` (+18 more) |
| Gate | 20 | `Gate: goose3-aau`, `Gate: goose3-60j`, `Gate: goose3-j8k` (+17 more) |
| Risk | 11 | `Risk: beads MCP tool instability`, `Risk: failed to fetch all.html sacp 2026-02-06T22:37:00`, `Risk: failed to fetch all.html agent-client-protocol 2026-02-06T22:37:00` (+8 more) |
| DependencyEdge | 9 | `DependencyEdge:imports:goose->goose-mcp`, `DependencyEdge:imports:goose-acp->goose`, `DependencyEdge:imports:goose-bench->goose` (+6 more) |
| Feature | 8 | `Feature: goose session agent auto mode selection`, `Feature: goose session fixed agent mode`, `Goose agent packages: remote-only catalog + new CLI` (+5 more) |
| task | 7 | `goose3-9uc`, `goose3-au7`, `goose3-ls4` (+4 more) |
| Phase | 6 | `Phase1_Navigation`, `Phase2_PromptBar`, `Phase3_Catalogs` (+3 more) |
| CodeAnchor | 5 | `goose-acp: GooseAcpAgent::build_session_agent`, `goose-acp: ModeRegistry`, `goose core: AgentPackage` (+2 more) |
| ExecutionFlow | 5 | `ExecutionFlow:cli:run`, `ExecutionFlow:http:session`, `ExecutionFlow:cli:subcommands` (+2 more) |
| TestSurface | 4 | `TestSurface:goose:tests`, `TestSurface:goose-cli:tests`, `TestSurface:goose-server:tests` (+1 more) |
| Specification | 4 | `ACP OpenAPI spec (i-am-bee/acp openapi.yaml)`, `A2A schema (specification/buf.gen.yaml)`, `A2A schema (specification/buf.yaml)` (+1 more) |
| Library | 3 | `Library:sacp`, `Library:agent-client-protocol`, `Library:agent-client-protocol@0.9.4` |
| WorkItem | 2 | `Goal: goose session parity with goose-acp-server agent packages/modes`, `Task: Shared builder for agent packages + modes` |
| Design | 2 | `Goose Mode Orchestrator Runtime`, `Design: Goose runtime architecture (Agent/Subagent/ACP/Recipes/SlashCommands)` |
| Spec | 2 | `Spec:AgentPackageLayout`, `Spec:AgentPackageManifest:v1` |
| Organization | 2 | `Linux Foundation`, `Google` |
| UIComponent | 2 | `NavigationSystem`, `PromptBar` |
| Constraint | 1 | `Beads daemon instability` |
| PlannedAPI | 1 | `goose core: agent package mode builder API` |
| Task | 1 | `Task: goose2-uv9` |
| Epic | 1 | `Epic: goose2-18t` |
| Plan | 1 | `RPI: Hybrid Orchestrator (Inline+Subagent) + Explain Trace` |
| RepoProfile | 1 | `RepoProfile:goose3` |
| ArchitectureDecision | 1 | `ArchitectureDecision:goose-acp-serial-lenses-via-session-modes` |
| API | 1 | `API:AgentPackageLoader` |
| Repo | 1 | `Repo:goose3` |
| Issue | 1 | `goose3-c4w` |
| workstream | 1 | `goose3-1g1-agent-package-cli` |
| component | 1 | `Agent package catalog` |
| Config | 1 | `Agent Package Catalog (Goose)` |
| Test | 1 | `Agent Package Remote Catalog Tests` |
| Project | 1 | `GooseProject` |
| Protocol | 1 | `Agent Communication Protocol (ACP)` |
| Crate | 1 | `agent-client-protocol (Rust crate)` |
| DocumentationSite | 1 | `ACP docs site (agentcommunicationprotocol.dev)` |
| Claim | 1 | `ACP is now part of A2A (claim)` |
| Repository | 1 | `A2A GitHub repository (a2aproject/A2A)` |
| Application | 1 | `GooseDesktop` |
| Architecture | 1 | `CatalogSystem` |
| BackendComponent | 1 | `SessionManager` |

### Relation Types

| Relation | Count |
|----------|-------|
| related_to | 626 |
| documents | 222 |
| defines | 165 |
| derived_from | 135 |
| defined_in | 126 |
| implements | 120 |
| depends_on | 75 |
| uses | 68 |
| located_at | 62 |
| validated_by | 52 |
| owns | 46 |
| documented_by | 43 |
| affects | 33 |
| implemented_by | 26 |
| belongs_to | 23 |
| contains | 21 |
| verifies | 20 |
| compares | 17 |
| blocks | 16 |
| has_evidence | 14 |
| touches | 12 |
| informs | 11 |
| owned_by | 11 |
| supports | 8 |
| describes | 7 |
| has_child | 7 |
| extends | 7 |
| routes_to | 7 |
| enables | 6 |
| references | 6 |
| modifies | 6 |
| handles | 5 |
| exposes | 5 |
| tracks | 5 |
| requires | 4 |
| returns | 4 |
| includes | 4 |
| integrates_with | 4 |
| will_contain | 4 |
| unblocks | 3 |
| implemented_in | 3 |
| is published in | 3 |
| is replaced by | 2 |
| constrains | 2 |
| is implemented in | 2 |
| blocked_by | 2 |
| delivered_by | 2 |
| hasEvidence | 2 |
| bridges_to | 2 |
| similar_to | 2 |
| validates | 2 |
| complements | 2 |
| delegates_to | 2 |
| orthogonal_to | 2 |
| refines | 2 |
| used_by | 2 |
| configures | 2 |
| reviews_implementation_of | 2 |
| specifies | 2 |
| can refer to | 2 |
| is disambiguated by | 2 |
| renders_inline | 2 |
| delivers | 1 |
| is part of | 1 |
| governs | 1 |
| guides | 1 |
| stores | 1 |
| migrates_to | 1 |
| blocked | 1 |
| conforms_to | 1 |
| located_in | 1 |
| competes_with | 1 |
| complementary_to | 1 |
| packages | 1 |
| informed_by | 1 |
| updated_by | 1 |
| converts | 1 |
| targets | 1 |
| discovers | 1 |
| exposed_via | 1 |
| wraps | 1 |
| creates | 1 |
| triggers | 1 |
| configured_by | 1 |
| produces | 1 |
| reviews_usage_of | 1 |
| states | 1 |
| publishes | 1 |
| is distinct from | 1 |
| risk_for | 1 |
| based_on | 1 |
| disambiguates | 1 |
| adapts_to | 1 |
| publishes_to | 1 |
| evaluates | 1 |
| queries | 1 |
| persists_in | 1 |
| groups_sessions_in | 1 |
| enhances | 1 |
| parallel_with | 1 |

## Key Domains

### A2A Protocol (50 entities)

- **ACP is now part of A2A (claim)** (Claim) — 2 observations
- **A2A_Protocol** (Component) — 19 observations
- **A2A_DataModel** (Component) — 15 observations
- **A2A_Operations** (Component) — 13 observations
- **A2A_ProtocolBindings** (Component) — 9 observations
- **A2A_Security** (Component) — 8 observations
- **A2A_Extensions** (Component) — 7 observations
- **A2A_JS_Implementation** (Component) — 22 observations
- **MindMap:DeepDive:A2A_vs_ACP:2026-02-16** (Concept) — 2 observations
- **A2A_Protocol_Spec** (Concept) — 12 observations
- **Decision_A2A_Integration_Strategy** (Decision) — 11 observations
- **ADR-ACP-over-A2A** (Decision) — 7 observations
- **ACP_vs_A2A_Comparison** (Decision) — 14 observations
- **decision:terminology-rules-acp-a2a** (Decision) — 4 observations
- **Decision:Goose:A2A_Integration_Shape** (Decision) — 4 observations
- **A2A_Library_Design_Principles** (Decision) — 17 observations
- **A2A_Crate_Architecture** (Decision) — 35 observations
- **Finding_A2A_AgentCard_Schema_V2** (Finding) — 9 observations
- **Finding_A2A_RustEcosystem** (Finding) — 6 observations
- **Finding_A2A_LinuxFoundation** (Finding) — 10 observations
- **Finding_A2A_AgentCatalog** (Finding) — 6 observations
- **Finding_A2A_MCP_Complementary** (Finding) — 7 observations
- **ProtocolAlignment-ACP-A2A** (Finding) — 9 observations
- **ACP-A2A-Merger** (Finding) — 8 observations
- **finding:a2a-google-lf** (Finding) — 4 observations
- **finding:a2a-key-concepts** (Finding) — 4 observations
- **Finding:Matrix:A2A_vs_ACP** (Finding) — 3 observations
- **Finding:A2A:WorkflowModel** (Finding) — 3 observations
- **A2A_Spec_vs_JS_Gaps** (Finding) — 9 observations
- **A2A_Goose_Integration_Surface** (Finding) — 10 observations
- **A2A_AgentCard** (Interface) — 9 observations
- **A2A_Task** (Interface) — 10 observations
- **A2A_Artifact** (Interface) — 7 observations
- **repopath:kg-seed-acp-a2a** (RepoPath) — 4 observations
- **RepoPath:/home/jmercier/codes/A2A** (RepoPath) — 2 observations
- **A2A_Crate_Implementation_Summary** (RepoPath) — 18 observations
- **Goose_A2A_Compat_Module** (RepoPath) — 9 observations
- **A2A GitHub repository (a2aproject/A2A)** (Repository) — 2 observations
- **Source_GoogleA2A** (Source) — 3 observations
- **i-am-bee discussion #5 (ACP joins forces with A2A)** (Source) — 2 observations
- **Linux Foundation press release (A2A launch)** (Source) — 2 observations
- **Google Developers Blog (Announcing A2A)** (Source) — 2 observations
- **src:a2a-readme** (Source) — 4 observations
- **src:acp-mcp-and-a2a** (Source) — 4 observations
- **src:cratesio-page-a2a-client** (Source) — 4 observations
- **src:google-a2a-blog** (Source) — 4 observations
- **src:lf-a2a-press** (Source) — 4 observations
- **A2A schema (specification/buf.gen.yaml)** (Specification) — 4 observations
- **A2A schema (specification/buf.yaml)** (Specification) — 4 observations
- **A2A schema (specification/a2a.proto)** (Specification) — 4 observations

### ACP Protocol (413 entities)

- **ArchitectureDecision:goose-acp-serial-lenses-via-session-modes** (ArchitectureDecision) — 3 observations
- **ACP is now part of A2A (claim)** (Claim) — 2 observations
- **goose-acp: GooseAcpAgent::build_session_agent** (CodeAnchor) — 1 observations
- **goose-acp: ModeRegistry** (CodeAnchor) — 1 observations
- **CodeArtifact:file:crates/goose-acp/src/lib.rs** (CodeArtifact) — 1 observations
- **Crate: goose-acp** (Component) — 4 observations
- **Crate: goose-acp** (Component) — 1 observations
- **Library: sacp** (Component) — 5 observations
- **Component:goose-acp** (Component) — 2 observations
- **Component:goose-acp:server** (Component) — 1 observations
- **crates/goose-acp** (Component) — 13 observations
- **Component:crate:goose-acp** (Component) — 3 observations
- **Component:ACPServer-AgentPackageIntegration** (Component) — 5 observations
- **ACP_AgentCommunicationProtocol** (Component) — 19 observations
- **GooseAcpAgent** (Component) — 11 observations
- **GooseAcpCrate** (Component) — 9 observations
- **CrateGooseAcp** (Component) — 5 observations
- **AcpServer** (Component) — 12 observations
- **ACP_Architecture** (Component) — 8 observations
- **ACP_Rust_Crate** (Component) — 22 observations
- **ACP_SessionMode** (Component) — 5 observations
- **Goose_ACP_Compat** (Component) — 9 observations
- **Concept: ACP** (Concept) — 3 observations
- **Concept: ACP Server** (Concept) — 1 observations
- **Concept: ACP Session** (Concept) — 5 observations
- **Concept: ACP Agent** (Concept) — 5 observations
- **Concept: ACP Mode** (Concept) — 5 observations
- **Concept: ACP Messages** (Concept) — 5 observations
- **Concept: ACP Transport** (Concept) — 5 observations
- **Concept: ACP Errors** (Concept) — 5 observations
- **Concept:ACPAgentServer** (Concept) — 1 observations
- **Concept:ACPBridge** (Concept) — 1 observations
- **ConceptDecision:ACPTopology** (Concept) — 1 observations
- **Concept:ACP-SessionModes** (Concept) — 4 observations
- **ACP_Protocol** (Concept) — 17 observations
- **ACP-Protocol** (Concept) — 34 observations
- **ACP_Composition_Patterns** (Concept) — 7 observations
- **ACP_Wrapping_Agents** (Concept) — 7 observations
- **ACP_MCP_Relationship** (Concept) — 6 observations
- **Agent Manifest (ACP)** (Concept) — 1 observations
- **ACP (acronym collision)** (Concept) — 4 observations
- **concept:acp-agent-client-protocol** (Concept) — 3 observations
- **concept:acp-agent-communication-protocol** (Concept) — 6 observations
- **MindMap:DeepDive:A2A_vs_ACP:2026-02-16** (Concept) — 2 observations
- **Decision: ACP implemented by goose-acp crate and invoked via goose-cli command** (Decision) — 1 observations
- **Decision:ACP:HostInGoosed** (Decision) — 2 observations
- **Decision:ACPConceptMappingSchema:v1** (Decision) — 4 observations
- **Decision:goose-acp:migrate:sacp-to-agent-client-protocol:v1** (Decision) — 3 observations
- **Decision_SACP_Migration** (Decision) — 8 observations
- **Guide_SACP_Migration** (Decision) — 11 observations
- **ADR-ACP-over-A2A** (Decision) — 7 observations
- **ADR-ServiceWiring-Via-ACP** (Decision) — 8 observations
- **ACP_vs_A2A_Comparison** (Decision) — 14 observations
- **decision:terminology-rules-acp-a2a** (Decision) — 4 observations
- **DependencyEdge:imports:goose-acp->goose** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-cli->goose-acp** (DependencyEdge) — 1 observations
- **Design: Goose runtime architecture (Agent/Subagent/ACP/Recipes/SlashCommands)** (Design) — 2 observations
- **ACP docs site (agentcommunicationprotocol.dev)** (DocumentationSite) — 2 observations
- **ExecutionFlow:rpc:acp-via-goosed** (ExecutionFlow) — 1 observations
- **Finding: ACP references (initial) 2026-02-06T20:33:00** (Finding) — 1 observations
- **Finding: ACP anchors 2026-02-06T20:39:00** (Finding) — 1 observations
- **Finding: ACP core files 2026-02-06T20:40:00** (Finding) — 1 observations
- **Finding: ACP implementation map 2026-02-06T20:42:00** (Finding) — 1 observations
- **Finding: docs.rs crawl summary sacp 2026-02-06T22:38:00** (Finding) — 1 observations
- **Finding: docs.rs crawl summary sacp 2026-02-06T22:37:00** (Finding) — 1 observations
- **Finding: docs.rs crawl summary sacp 2026-02-06T22:44:00** (Finding) — 1 observations
- **Finding: docs.rs all.html ingestion sacp 2026-02-06T23:05:00** (Finding) — 1 observations
- **Finding: docs.rs ingestion status sacp 2026-02-06T23:40:00** (Finding) — 1 observations
- **Finding:ACP-Agent-Session-Mode:Extraction** (Finding) — 3 observations
- **Finding:Agent-Session-Mode (Goose ACP)** (Finding) — 19 observations
- **Finding:ACP Agent/Session/Mode focus** (Finding) — 3 observations
- **Finding:ACP Session Flow Map** (Finding) — 5 observations
- **Finding:ACP Handler Flow Map (complete)** (Finding) — 12 observations
- **Finding:GooseAcpSession internal model** (Finding) — 7 observations
- **Finding:ACP Mode (agent-client-protocol)** (Finding) — 19 observations
- **Finding:CLI:AcpCommandRunsSeparateServer** (Finding) — 3 observations
- **Finding:ACP:Migrate:sacp-to-acp:Scope:v0** (Finding) — 3 observations
- **Finding:goose-acp:migration:sacp-to-acp:step1:e9b99f7418a81c361450aedc057f4b987198c7bf

e9b99f7418a81c361450aedc057f4b987198c7bf** (Finding) — 2 observations
- **Finding:goose-acp:migration:sacp-to-acp:proto-shim:wip** (Finding) — 2 observations
- **Finding:goose-acp:migration:sacp-to-acp:proto-fixes:6c9fb73966198bffde951afd340a42fd378df3ef** (Finding) — 2 observations
- **Finding_ACP_Manifest_V2** (Finding) — 8 observations
- **Finding_SACP_vs_ACP_Crate** (Finding) — 10 observations
- **Finding_ACPClient_Registry_Schema** (Finding) — 9 observations
- **Finding_ACP_NotSend_Constraint** (Finding) — 11 observations
- **Guide_SACP_Migration_Detailed** (Finding) — 20 observations
- **ProtocolAlignment-ACP-A2A** (Finding) — 9 observations
- **ACP-Architecture-Patterns** (Finding) — 9 observations
- **ACP-AgentManifest-Spec** (Finding) — 7 observations
- **ACP-Discovery-Methods** (Finding) — 7 observations
- **ACP-Compose-Patterns** (Finding) — 7 observations
- **ACP-MCP-Bridge** (Finding) — 9 observations
- **ACP-A2A-Merger** (Finding) — 8 observations
- **finding:acp-acronym-collision** (Finding) — 4 observations
- **finding:acp-docs-agent-to-agent** (Finding) — 4 observations
- **finding:beeai-acp-openapi-title** (Finding) — 4 observations
- **Finding:Matrix:A2A_vs_ACP** (Finding) — 3 observations
- **Finding:ACP:Scope** (Finding) — 3 observations
- **Interface: ACP server** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::return** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::with** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::return** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::[Errno** (Interface) — 1 observations
- **Interface: sacp::(most** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::return** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::with** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::File** (Interface) — 1 observations
- **Interface: sacp::return** (Interface) — 1 observations
- **Interface: sacp::~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^** (Interface) — 1 observations
- **Interface: sacp::[Errno** (Interface) — 1 observations
- **Interface: sacp::Diff** (Interface) — 1 observations
- **Interface: sacp::EmbeddedResource** (Interface) — 1 observations
- **Interface: sacp::EnvVariable** (Interface) — 1 observations
- **Interface: sacp::Error** (Interface) — 1 observations
- **Interface: sacp::ExtNotification** (Interface) — 1 observations
- **Interface: sacp::ExtRequest** (Interface) — 1 observations
- **Interface: sacp::ExtResponse** (Interface) — 1 observations
- **Interface: sacp::FileSystemCapability** (Interface) — 1 observations
- **Interface: sacp::HttpHeader** (Interface) — 1 observations
- **Interface: sacp::ImageContent** (Interface) — 1 observations
- **Interface: sacp::Implementation** (Interface) — 1 observations
- **Interface: sacp::InitializeProxyRequest** (Interface) — 1 observations
- **Interface: sacp::InitializeRequest** (Interface) — 1 observations
- **Interface: sacp::InitializeResponse** (Interface) — 1 observations
- **Interface: sacp::JsonRpcMessage** (Interface) — 1 observations
- **Interface: sacp::KillTerminalCommandRequest** (Interface) — 1 observations
- **Interface: sacp::KillTerminalCommandResponse** (Interface) — 1 observations
- **Interface: sacp::LoadSessionRequest** (Interface) — 1 observations
- **Interface: sacp::LoadSessionResponse** (Interface) — 1 observations
- **Interface: sacp::McpCapabilities** (Interface) — 1 observations
- **Interface: sacp::McpConnectRequest** (Interface) — 1 observations
- **Interface: sacp::McpConnectResponse** (Interface) — 1 observations
- **Interface: sacp::McpDisconnectNotification** (Interface) — 1 observations
- **Interface: sacp::McpOverAcpMessage** (Interface) — 1 observations
- **Interface: sacp::McpServerHttp** (Interface) — 1 observations
- **Interface: sacp::McpServerSse** (Interface) — 1 observations
- **Interface: sacp::McpServerStdio** (Interface) — 1 observations
- **Interface: sacp::NewSessionRequest** (Interface) — 1 observations
- **Interface: sacp::NewSessionResponse** (Interface) — 1 observations
- **Interface: sacp::Notification** (Interface) — 1 observations
- **Interface: sacp::PermissionOption** (Interface) — 1 observations
- **Interface: sacp::PermissionOptionId** (Interface) — 1 observations
- **Interface: sacp::Plan** (Interface) — 1 observations
- **Interface: sacp::PlanEntry** (Interface) — 1 observations
- **Interface: sacp::PromptCapabilities** (Interface) — 1 observations
- **Interface: sacp::PromptRequest** (Interface) — 1 observations
- **Interface: sacp::PromptResponse** (Interface) — 1 observations
- **Interface: sacp::ProtocolVersion** (Interface) — 1 observations
- **Interface: sacp::RawValue** (Interface) — 1 observations
- **Interface: sacp::ReadTextFileRequest** (Interface) — 1 observations
- **Interface: sacp::ReadTextFileResponse** (Interface) — 1 observations
- **Interface: sacp::ReleaseTerminalRequest** (Interface) — 1 observations
- **Interface: sacp::ReleaseTerminalResponse** (Interface) — 1 observations
- **Interface: sacp::Request** (Interface) — 1 observations
- **Interface: sacp::RequestPermissionRequest** (Interface) — 1 observations
- **Interface: sacp::RequestPermissionResponse** (Interface) — 1 observations
- **Interface: sacp::ResourceLink** (Interface) — 1 observations
- **Interface: sacp::SelectedPermissionOutcome** (Interface) — 1 observations
- **Interface: sacp::SessionCapabilities** (Interface) — 1 observations
- **Interface: sacp::SessionId** (Interface) — 1 observations
- **Interface: sacp::SessionMode** (Interface) — 1 observations
- **Interface: sacp::SessionModeId** (Interface) — 1 observations
- **Interface: sacp::SessionModeState** (Interface) — 1 observations
- **Interface: sacp::SessionNotification** (Interface) — 1 observations
- **Interface: sacp::SetSessionModeRequest** (Interface) — 1 observations
- **Interface: sacp::SetSessionModeResponse** (Interface) — 1 observations
- **Interface: sacp::SuccessorMessage** (Interface) — 1 observations
- **Interface: sacp::Terminal** (Interface) — 1 observations
- **Interface: sacp::TerminalExitStatus** (Interface) — 1 observations
- **Interface: sacp::TerminalId** (Interface) — 1 observations
- **Interface: sacp::TerminalOutputRequest** (Interface) — 1 observations
- **Interface: sacp::TerminalOutputResponse** (Interface) — 1 observations
- **Interface: sacp::TextContent** (Interface) — 1 observations
- **Interface: sacp::TextResourceContents** (Interface) — 1 observations
- **Interface: sacp::ToolCall** (Interface) — 1 observations
- **Interface: sacp::ToolCallId** (Interface) — 1 observations
- **Interface: sacp::ToolCallLocation** (Interface) — 1 observations
- **Interface: sacp::ToolCallUpdate** (Interface) — 1 observations
- **Interface: sacp::ToolCallUpdateFields** (Interface) — 1 observations
- **Interface: sacp::UnstructuredCommandInput** (Interface) — 1 observations
- **Interface: sacp::WaitForTerminalExitRequest** (Interface) — 1 observations
- **Interface: sacp::WaitForTerminalExitResponse** (Interface) — 1 observations
- **Interface: sacp::WriteTextFileRequest** (Interface) — 1 observations
- **Interface: sacp::WriteTextFileResponse** (Interface) — 1 observations
- **Interface: sacp::IntoMaybeUndefined** (Interface) — 1 observations
- **Interface: sacp::IntoOption** (Interface) — 1 observations
- **Interface: sacp::Side** (Interface) — 1 observations
- **Interface: sacp::Meta** (Interface) — 1 observations
- **Interface: sacp::Result** (Interface) — 1 observations
- **Interface: sacp::ActiveSession** (Interface) — 1 observations
- **Interface: sacp::Blocking** (Interface) — 1 observations
- **Interface: sacp::ByteStreams** (Interface) — 1 observations
- **Interface: sacp::ChainResponder** (Interface) — 1 observations
- **Interface: sacp::Channel** (Interface) — 1 observations
- **Interface: sacp::JrConnection** (Interface) — 1 observations
- **Interface: sacp::JrConnectionBuilder** (Interface) — 1 observations
- **Interface: sacp::JrConnectionCx** (Interface) — 1 observations
- **Interface: sacp::JrRequestCx** (Interface) — 1 observations
- **Interface: sacp::JrResponse** (Interface) — 1 observations
- **Interface: sacp::Lines** (Interface) — 1 observations
- **Interface: sacp::McpAcpTransport** (Interface) — 1 observations
- **Interface: sacp::NonBlocking** (Interface) — 1 observations
- **Interface: sacp::NullHandler** (Interface) — 1 observations
- **Interface: sacp::NullResponder** (Interface) — 1 observations
- **Interface: sacp::SessionBuilder** (Interface) — 1 observations
- **Interface: sacp::UntypedMessage** (Interface) — 1 observations
- **Interface: sacp::IntoHandled** (Interface) — 1 observations
- **Interface: sacp::JrMessage** (Interface) — 1 observations
- **Interface: sacp::JrMessageHandler** (Interface) — 1 observations
- **Interface: sacp::Output** (Interface) — 1 observations
- **Interface:sacp:struct:ActiveSession** (Interface) — 6 observations
- **Interface:sacp:struct:JrRequestCx** (Interface) — 5 observations
- **Interface:sacp:struct:JrResponse** (Interface) — 5 observations
- **Interface:sacp:struct:McpAcpTransport** (Interface) — 5 observations
- **Interface:sacp:struct:SessionBuilder** (Interface) — 6 observations
- **Interface:sacp:struct:UntypedMessage** (Interface) — 5 observations
- **Interface:sacp:enum:AgentNotification** (Interface) — 5 observations
- **Interface:sacp:enum:AgentRequest** (Interface) — 5 observations
- **Interface:sacp:enum:AgentResponse** (Interface) — 5 observations
- **Interface:sacp:enum:ClientRequest** (Interface) — 5 observations
- **Interface:sacp:enum:ClientResponse** (Interface) — 5 observations
- **Interface:sacp:enum:MessageCx** (Interface) — 6 observations
- **Interface:sacp:enum:SessionMessage** (Interface) — 5 observations
- **Interface:sacp:trait:JrMessage** (Interface) — 5 observations
- **Interface:sacp:trait:JrMessageHandler** (Interface) — 6 observations
- **Interface:sacp:trait:JrRequest** (Interface) — 5 observations
- **Interface:sacp:trait:JrResponsePayload** (Interface) — 5 observations
- **Interface:sacp:trait:SessionBlockState** (Interface) — 5 observations
- **Interface:sacp:macro:on_receive_message** (Interface) — 5 observations
- **Interface:sacp:macro:on_receive_request** (Interface) — 5 observations
- **Interface:sacp:unknown:ByteStreams** (Interface) — 2 observations
- **Interface:sacp:unknown:Handled** (Interface) — 2 observations
- **Interface:sacp:unknown:JrConnectionCx** (Interface) — 2 observations
- **Interface:sacp:unknown:JrMessageHandler** (Interface) — 2 observations
- **Interface:sacp:unknown:MessageCx** (Interface) — 2 observations
- **Interface:sacp:struct:SessionId** (Interface) — 3 observations
- **Interface:sacp:struct:SessionNotification** (Interface) — 3 observations
- **Interface:sacp:enum:SessionUpdate** (Interface) — 3 observations
- **Interface:sacp:struct:NewSessionRequest** (Interface) — 3 observations
- **Interface:sacp:struct:NewSessionResponse** (Interface) — 3 observations
- **Interface:sacp:struct:LoadSessionRequest** (Interface) — 3 observations
- **Interface:sacp:struct:LoadSessionResponse** (Interface) — 3 observations
- **Interface:sacp:struct:InitializeRequest** (Interface) — 3 observations
- **Interface:sacp:struct:InitializeResponse** (Interface) — 3 observations
- **Interface:sacp:struct:PromptRequest** (Interface) — 3 observations
- **Interface:sacp:struct:PromptResponse** (Interface) — 3 observations
- **Interface:sacp:struct:CancelNotification** (Interface) — 3 observations
- **Interface:sacp:struct:AgentCapabilities** (Interface) — 3 observations
- **Interface:sacp:struct:McpServer** (Interface) — 3 observations
- **Interface:sacp:struct:ByteStreams** (Interface) — 3 observations
- **Interface:sacp:struct:JrConnectionCx** (Interface) — 3 observations
- **Interface:sacp:struct:schema/struct.SessionId.html** (Interface) — 6 observations
- **Interface:sacp:struct:struct.SessionBuilder.html** (Interface) — 6 observations
- **Interface:sacp:struct:struct.ActiveSession.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.AgentCapabilities.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.InitializeRequest.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.InitializeResponse.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.NewSessionRequest.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.NewSessionResponse.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.LoadSessionRequest.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.LoadSessionResponse.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.PromptRequest.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.PromptResponse.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.CancelNotification.html** (Interface) — 6 observations
- **Interface:sacp:struct:schema/struct.SessionNotification.html** (Interface) — 6 observations
- **Interface:sacp:enum:schema/enum.SessionUpdate.html** (Interface) — 6 observations
- **Interface:sacp:struct:struct.ByteStreams.html** (Interface) — 6 observations
- **Interface:sacp:struct:struct.JrConnectionCx.html** (Interface) — 6 observations
- **Interface:sacp:trait:trait.JrMessageHandler.html** (Interface) — 6 observations
- **Interface:sacp:enum:enum.MessageCx.html** (Interface) — 6 observations
- **Interface:sacp:struct:link/struct.AgentToClient.html** (Interface) — 6 observations
- **Interface:sacp:struct:link/struct.ClientToAgent.html** (Interface) — 6 observations
- **Interface:docsrs:sacp:struct:schema/struct.AgentCapabilities.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:link/struct.AgentToClient.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:link/struct.ClientToAgent.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:trait:trait.JrMessageHandler.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SessionId.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.NewSessionRequest.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.NewSessionResponse.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.LoadSessionRequest.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.LoadSessionResponse.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.InitializeRequest.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.InitializeResponse.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.PromptRequest.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.PromptResponse.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.CancelNotification.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:struct.ActiveSession.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:struct.SessionBuilder.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SessionMode.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SessionModeId.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SessionModeState.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SetSessionModeRequest.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.SetSessionModeResponse.html** (Interface) — 9 observations
- **Interface:docsrs:sacp:struct:schema/struct.CurrentModeUpdate.html** (Interface) — 9 observations
- **Interface:GooseACPHandler:on_cancel** (Interface) — 3 observations
- **Interface:ACP:sacp:CancelNotification** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_initialize** (Interface) — 4 observations
- **Interface:ACP:sacp:InitializeRequest** (Interface) — 2 observations
- **Interface:ACP:sacp:InitializeResponse** (Interface) — 2 observations
- **Interface:GooseACPHandler:on_new_session** (Interface) — 4 observations
- **Interface:ACP:sacp:NewSessionRequest** (Interface) — 4 observations
- **Interface:ACP:sacp:NewSessionResponse** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_load_session** (Interface) — 4 observations
- **Interface:ACP:sacp:LoadSessionRequest** (Interface) — 4 observations
- **Interface:ACP:sacp:LoadSessionResponse** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_prompt** (Interface) — 4 observations
- **Interface:ACP:sacp:PromptRequest** (Interface) — 4 observations
- **Interface:ACP:sacp:PromptResponse** (Interface) — 4 observations
- **Interface:GooseAcpSession** (Interface) — 4 observations
- **Interface:GooseAcpAgent** (Interface) — 4 observations
- **Interface:GooseAcpSessionsStore** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:SessionMode** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:SessionModeId** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:SessionModeState** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:SetSessionModeRequest** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:SetSessionModeResponse** (Interface) — 3 observations
- **Interface:ACP:agent-client-protocol:CurrentModeUpdate** (Interface) — 3 observations
- **Interface:rpc:acp-agent** (Interface) — 1 observations
- **ACP_OpenAPI_Spec** (Interface) — 13 observations
- **ACP_AgentManifest** (Interface) — 10 observations
- **ACPSessionModeSpec** (Interface) — 7 observations
- **Library:sacp** (Library) — 21 observations
- **Agent Communication Protocol (ACP)** (Protocol) — 2 observations
- **RepoPath: crates/goose-acp** (RepoPath) — 4 observations
- **RepoPath: crates/goose-acp** (RepoPath) — 1 observations
- **RepoPath: crates/goose-acp/src/server.rs** (RepoPath) — 9 observations
- **RepoPath: crates/goose-acp/src/transport.rs** (RepoPath) — 8 observations
- **RepoPath: crates/goose-acp/src/transport/http.rs** (RepoPath) — 7 observations
- **RepoPath: crates/goose-acp/src/transport/websocket.rs** (RepoPath) — 7 observations
- **RepoPath: crates/goose-acp/Cargo.toml** (RepoPath) — 2 observations
- **RepoPath:crates/goose-acp/src/server.rs** (RepoPath) — 28 observations
- **RepoPath:/tmp/plan-goose3-v6f-acp-mapping-schema.md** (RepoPath) — 1 observations
- **RepoPath_GooseACP** (RepoPath) — 5 observations
- **GooseCode_ACP** (RepoPath) — 10 observations
- **goose_acp_server_rs** (RepoPath) — 4 observations
- **repopath:goose-arch-unified-acp-api** (RepoPath) — 4 observations
- **repopath:kg-seed-acp-a2a** (RepoPath) — 4 observations
- **Risk: failed to fetch all.html sacp 2026-02-06T22:37:00** (Risk) — 1 observations
- **Risk: failed to fetch all.html sacp 2026-02-06T22:44:00** (Risk) — 1 observations
- **Source: rg acp 2026-02-06T20:33:00** (Source) — 1 observations
- **Source: hitAcp 2026-02-06T20:39:00** (Source) — 1 observations
- **Source: snippet acp 2026-02-06T20:39:00** (Source) — 1 observations
- **Source: snippet goose-acp server.rs 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: snippet goose-acp transport.rs 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: rg acp entry 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: overview acp 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: rg acp 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: snippet goose-acp server.rs 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: docs.rs sacp https://docs.rs/sacp/latest/sacp/** (Source) — 1 observations
- **Source: docs.rs sacp https://docs.rs/sacp/latest/sacp/index.html** (Source) — 1 observations
- **Source: docs.rs sacp https://docs.rs/sacp/latest/sacp/all.html** (Source) — 1 observations
- **Source: docs.rs sacp all.html 2026-02-06T22:38:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:38:00** (Source) — 1 observations
- **Source: goose-acp uses agent-client-protocol 2026-02-06T22:38:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:37:00** (Source) — 1 observations
- **Source: goose-acp mentions agent-client-protocol 2026-02-06T22:37:00** (Source) — 1 observations
- **Source: goose-acp Cargo.toml 2026-02-06T22:44:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:44:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp Cargo.toml 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp server.rs head 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: docs.rs sacp index https://docs.rs/sacp/latest/sacp/** (Source) — 1 observations
- **Source: docs.rs sacp all.html https://docs.rs/sacp/latest/sacp/all.html** (Source) — 1 observations
- **Source: docs.rs sacp url list 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: docs.rs sacp samples 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T23:40:00** (Source) — 1 observations
- **Source: docs.rs sacp index 2026-02-06T23:40:00** (Source) — 1 observations
- **Source: docs.rs sacp all.html 2026-02-06T23:40:00** (Source) — 1 observations
- **Source: docs.rs sacp url list 2026-02-06T23:40:00** (Source) — 1 observations
- **Source: docs.rs sacp samples 2026-02-06T23:40:00** (Source) — 1 observations
- **Source:docsrs:sacp:all.html** (Source) — 2 observations
- **Source:docsrs:sacp:urls.txt** (Source) — 1 observations
- **Source:docsrs:sacp:manifest.jsonl** (Source) — 1 observations
- **Source:docsrs:sacp:concept_extract.jsonl** (Source) — 1 observations
- **Source:artifact:/tmp/acp_agent_session_mode_focus.json** (Source) — 2 observations
- **Source:artifact:/tmp/acp_session_flow_map.md** (Source) — 1 observations
- **Source:artifact:/tmp/acp_session_flow_map.json** (Source) — 1 observations
- **Source:local:/tmp/research-goose3-5ik-sacp-to-agent-client-protocol.md** (Source) — 2 observations
- **Source:local:/tmp/plan-goose3-bjg-sacp-to-agent-client-protocol.md** (Source) — 2 observations
- **Source_ACP_IBM** (Source) — 4 observations
- **i-am-bee discussion #5 (ACP joins forces with A2A)** (Source) — 2 observations
- **src:acp-mcp-and-a2a** (Source) — 4 observations
- **src:beeai-acp-openapi** (Source) — 4 observations
- **ACP OpenAPI spec (i-am-bee/acp openapi.yaml)** (Specification) — 2 observations
- **TestEvidence:acp-roadmap:research:2026-02-07T23:19:00** (TestEvidence) — 14 observations
- **TestEvidence:acp-roadmap:topology-ui:2026-02-07T23:25:00** (TestEvidence) — 22 observations
- **TestEvidence:acp-roadmap:plan:2026-02-08T00:06:00** (TestEvidence) — 12 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint:2026-02-08T00:22:00** (TestEvidence) — 17 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint-fix:2026-02-08T00:26:00** (TestEvidence) — 12 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint-verified:2026-02-08T00:32:00** (TestEvidence) — 6 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:53:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:54:00** (TestEvidence) — 15 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:57:00** (TestEvidence) — 13 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test-verified:2026-02-08T00:57:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-smoke-verified:2026-02-08T01:02:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T01:42:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:00:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:20:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:verify:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:48:00** (TestEvidence) — 6 observations
- **TestEvidence:acp-roadmap:impl:agent-packages-manifest-v1-verified:2026-02-08T08:25:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:impl:agent-packages-schema-v1-verified:2026-02-08T08:30:00** (TestEvidence) — 5 observations
- **Goal: goose session parity with goose-acp-server agent packages/modes** (WorkItem) — 15 observations

### Goose Architecture (505 entities)

- **GooseDesktop** (Application) — 11 observations
- **ArchitectureDecision:goose-acp-serial-lenses-via-session-modes** (ArchitectureDecision) — 3 observations
- **BeadsEpic: goose3-dcq** (BeadsEpic) — 2 observations
- **BeadsEpic: goose3-daa** (BeadsEpic) — 2 observations
- **BeadsEpic: goose3-hgx** (BeadsEpic) — 5 observations
- **BeadsEpic:goose3-9l5** (BeadsEpic) — 3 observations
- **BeadsEpic:goose3-zw6** (BeadsEpic) — 3 observations
- **BeadsEpic:goose3-c1q** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-ts8** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-mly** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-tuh** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-vjv** (BeadsEpic) — 1 observations
- **goose3-vjv** (BeadsEpic) — 4 observations
- **goose3-xts** (BeadsEpic) — 5 observations
- **goose3-70l** (BeadsEpic) — 5 observations
- **goose3-wlz** (BeadsEpic) — 45 observations
- **BeadsEpic:goose3-wlz** (BeadsEpic) — 2 observations
- **Goose-MetaOrchestrator** (BeadsEpic) — 9 observations
- **BeadsTask: goose3-61d** (BeadsTask) — 2 observations
- **BeadsTask: goose3-vr0** (BeadsTask) — 2 observations
- **BeadsTask: goose3-mf0** (BeadsTask) — 2 observations
- **BeadsTask: goose3-dhr** (BeadsTask) — 2 observations
- **BeadsTask: goose3-3i0** (BeadsTask) — 1 observations
- **BeadsTask: goose3-2w8** (BeadsTask) — 1 observations
- **BeadsTask: goose3-tqp** (BeadsTask) — 2 observations
- **BeadsTask: goose3-fyj** (BeadsTask) — 2 observations
- **BeadsTask:goose3-c33** (BeadsTask) — 1 observations
- **BeadsTask:goose3-om3** (BeadsTask) — 1 observations
- **BeadsTask:goose3-1iz** (BeadsTask) — 1 observations
- **BeadsTask:goose3-5mp** (BeadsTask) — 1 observations
- **BeadsTask:goose3-3ho** (BeadsTask) — 2 observations
- **BeadsTask:goose3-9af** (BeadsTask) — 2 observations
- **BeadsTask:goose3-6m3** (BeadsTask) — 2 observations
- **BeadsTask:goose3-4jw** (BeadsTask) — 3 observations
- **BeadsTask:goose3-fyj** (BeadsTask) — 2 observations
- **BeadsTask:goose3-kkt** (BeadsTask) — 2 observations
- **BeadsTask:goose3-q2g** (BeadsTask) — 2 observations
- **BeadsTask:goose3-c2v** (BeadsTask) — 2 observations
- **BeadsTask:goose3-2qy** (BeadsTask) — 2 observations
- **BeadsTask:goose3-5na** (BeadsTask) — 2 observations
- **BeadsTask:goose3-5ik** (BeadsTask) — 2 observations
- **BeadsTask:goose3-bjg** (BeadsTask) — 2 observations
- **goose3-2rz** (BeadsTask) — 63 observations
- **goose3-g3n** (BeadsTask) — 5 observations
- **goose3-35e** (BeadsTask) — 5 observations
- **goose3-4iu** (BeadsTask) — 2 observations
- **goose3-95o** (BeadsTask) — 2 observations
- **goose3-h6u** (BeadsTask) — 2 observations
- **goose3-8v1** (BeadsTask) — 7 observations
- **goose3-vvf** (BeadsTask) — 10 observations
- **goose3-eyb** (BeadsTask) — 7 observations
- **goose3-zpg** (BeadsTask) — 5 observations
- **goose3-2b0** (BeadsTask) — 3 observations
- **BeadsTask:goose3-eyb** (BeadsTask) — 4 observations
- **BeadsTask:goose3-zpg** (BeadsTask) — 1 observations
- **BeadsTask:goose3-2b0** (BeadsTask) — 1 observations
- **BeadsTask:goose3-350** (BeadsTask) — 6 observations
- **BeadsTask:goose3-8cg** (BeadsTask) — 6 observations
- **BeadsTask:goose3-9uc** (BeadsTask) — 55 observations
- **BeadsTask:goose3-c4w** (BeadsTask) — 15 observations
- **BeadsTask:goose3-8ba** (BeadsTask) — 3 observations
- **BeadsTask:goose3-ooa** (BeadsTask) — 3 observations
- **BeadsTask:goose3-rq7** (BeadsTask) — 3 observations
- **goose-acp: GooseAcpAgent::build_session_agent** (CodeAnchor) — 1 observations
- **goose-acp: ModeRegistry** (CodeAnchor) — 1 observations
- **goose core: AgentPackage** (CodeAnchor) — 1 observations
- **goose core: AgentProfileState** (CodeAnchor) — 1 observations
- **goose core: Agent::persist_session_extension_state** (CodeAnchor) — 1 observations
- **CodeArtifact:file:crates/goose/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/main.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/main.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/routes/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/tool_monitor.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/tool_inspection.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/providers/retry.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/providers/xai.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/providers/tetrate.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/providers/lead_worker.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/signal.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/scenario_tests/scenario_runner.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/scenario_tests/provider_configs.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/cli.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/scenario_tests/message_generator.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/logging.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-cli/src/session/output.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/error.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/configuration.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/state.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/tutorial/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/lang.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/shell.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/editor_models/relace_editor.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/mcp_server_runner.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/editor_models/openai_compatible_editor.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/text_editor.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/editor_models/morphllm_editor.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/editor_models/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/computercontroller/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-acp/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-bench/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-test/src/lib.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/agents/agent.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/providers/base.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose/src/session/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/openapi.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-server/src/routes/session.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/developer/rmcp_developer.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:file:crates/goose-mcp/src/memory/mod.rs** (CodeArtifact) — 1 observations
- **CodeArtifact:crates/goose/src/agent_packages/mod.rs** (CodeArtifact) — 4 observations
- **Crate: goose** (Component) — 4 observations
- **Crate: goose-acp** (Component) — 4 observations
- **Crate: goose-bench** (Component) — 2 observations
- **Crate: goose-cli** (Component) — 4 observations
- **Crate: goose-mcp** (Component) — 4 observations
- **Crate: goose-server** (Component) — 4 observations
- **Crate: goose-test** (Component) — 3 observations
- **Crate: goose** (Component) — 1 observations
- **Crate: goose-acp** (Component) — 1 observations
- **Crate: goose-bench** (Component) — 1 observations
- **Crate: goose-cli** (Component) — 1 observations
- **Crate: goose-mcp** (Component) — 1 observations
- **Crate: goose-server** (Component) — 1 observations
- **Crate: goose-test** (Component) — 1 observations
- **Component:goose-acp** (Component) — 2 observations
- **Component:goose:agent_packages** (Component) — 2 observations
- **Component:goose-server:routes:agent_packages** (Component) — 2 observations
- **Component:goose-cli:agent_packages_commands** (Component) — 2 observations
- **Component:goose-acp:server** (Component) — 1 observations
- **crates/goose-acp** (Component) — 13 observations
- **Component:crate:goose** (Component) — 3 observations
- **Component:crate:goose-mcp** (Component) — 4 observations
- **Component:crate:goose-acp** (Component) — 3 observations
- **Component:crate:goose-bench** (Component) — 3 observations
- **Component:crate:goose-cli** (Component) — 4 observations
- **Component:crate:goose-server** (Component) — 3 observations
- **Component:crate:goose-test** (Component) — 3 observations
- **GooseCodebase_Architecture** (Component) — 13 observations
- **GooseAcpAgent** (Component) — 11 observations
- **GooseAcpCrate** (Component) — 9 observations
- **GooseAgent** (Component) — 7 observations
- **GooseServer-AgentRoutes** (Component) — 8 observations
- **GooseCLI-Agents** (Component) — 5 observations
- **CrateGoose** (Component) — 12 observations
- **CrateGooseCli** (Component) — 9 observations
- **CrateGooseServer** (Component) — 8 observations
- **CrateGooseMcp** (Component) — 9 observations
- **CrateGooseAcp** (Component) — 5 observations
- **GooseAgentModes** (Component) — 12 observations
- **GooseCrate** (Component) — 14 observations
- **GooseCliCrate** (Component) — 8 observations
- **GooseServerCrate** (Component) — 10 observations
- **GooseMcpCrate** (Component) — 8 observations
- **GooseAgent_PlannerMode** (Component) — 5 observations
- **component:goose-cli** (Component) — 3 observations
- **component:goose-core** (Component) — 3 observations
- **component:goose-server** (Component) — 3 observations
- **GooseAgentArchitecture** (Component) — 6 observations
- **GooseAgentPersonaDesign** (Component) — 6 observations
- **Goose_ACP_Compat** (Component) — 9 observations
- **Goose_AgentManager** (Component) — 8 observations
- **Goose_Crate_Structure** (Component) — 9 observations
- **Concept: GooseAgent** (Concept) — 1 observations
- **GooseAppsSystem** (Concept) — 6 observations
- **Agent Package Catalog (Goose)** (Config) — 10 observations
- **Decision: goose session --mode requires --agent** (Decision) — 1 observations
- **Decision: Treat /plan as a CLI slash command (goose-cli)** (Decision) — 1 observations
- **Decision: Model subagent as a tool + SubAgent session type (goose core)** (Decision) — 1 observations
- **Decision: ACP implemented by goose-acp crate and invoked via goose-cli command** (Decision) — 1 observations
- **Decision:ACP:HostInGoosed** (Decision) — 2 observations
- **Decision:goose-acp:migrate:sacp-to-agent-client-protocol:v1** (Decision) — 3 observations
- **AgentRegistry_GooseRelevance** (Decision) — 10 observations
- **AgentRegistry_GooseRecommendations_V2** (Decision) — 10 observations
- **Decision_GooseAgentFormat_V2** (Decision) — 10 observations
- **Decision_GooseAsMetaOrchestrator** (Decision) — 10 observations
- **Decision:Goose:A2A_Integration_Shape** (Decision) — 4 observations
- **DependencyEdge:imports:goose->goose-mcp** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-acp->goose** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-bench->goose** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-cli->goose** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-cli->goose-acp** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-cli->goose-bench** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-cli->goose-mcp** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-server->goose** (DependencyEdge) — 1 observations
- **DependencyEdge:imports:goose-server->goose-mcp** (DependencyEdge) — 1 observations
- **Goose Mode Orchestrator Runtime** (Design) — 4 observations
- **Design: Goose runtime architecture (Agent/Subagent/ACP/Recipes/SlashCommands)** (Design) — 2 observations
- **Epic: goose2-18t** (Epic) — 1 observations
- **ExecutionFlow:rpc:acp-via-goosed** (ExecutionFlow) — 1 observations
- **Feature: goose session agent auto mode selection** (Feature) — 3 observations
- **Feature: goose session fixed agent mode** (Feature) — 2 observations
- **Goose agent packages: remote-only catalog + new CLI** (Feature) — 24 observations
- **Finding: Beads DB missing initially in goose3** (Finding) — 1 observations
- **Finding: Beads DB initialized in goose3** (Finding) — 1 observations
- **Finding: RPI gate dependency chain created for goose3 epic** (Finding) — 1 observations
- **Finding:Agent-Session-Mode (Goose ACP)** (Finding) — 19 observations
- **Finding:GooseAcpSession internal model** (Finding) — 7 observations
- **Finding:goosed:RoutesAndOpenApiComposition** (Finding) — 2 observations
- **Finding:goosed:IntegrationPoint:routes+openapi** (Finding) — 2 observations
- **Finding:goose-acp:migration:sacp-to-acp:step1:e9b99f7418a81c361450aedc057f4b987198c7bf

e9b99f7418a81c361450aedc057f4b987198c7bf** (Finding) — 2 observations
- **Finding:goose-acp:migration:sacp-to-acp:proto-shim:wip** (Finding) — 2 observations
- **Finding:goose-acp:migration:sacp-to-acp:proto-fixes:6c9fb73966198bffde951afd340a42fd378df3ef** (Finding) — 2 observations
- **Finding_GooseExistingTaxonomy** (Finding) — 10 observations
- **Finding_GooseMode_Current** (Finding) — 8 observations
- **A2A_Goose_Integration_Surface** (Finding) — 10 observations
- **Gate: goose3-aau** (Gate) — 3 observations
- **Gate: goose3-60j** (Gate) — 2 observations
- **Gate: goose3-j8k** (Gate) — 2 observations
- **Gate: goose3-xzf** (Gate) — 2 observations
- **Gate: goose3-w7u** (Gate) — 1 observations
- **Gate: goose3-8ac** (Gate) — 1 observations
- **Gate: goose3-uvv** (Gate) — 1 observations
- **Gate:goose3-p6l** (Gate) — 1 observations
- **Gate:goose3-du1** (Gate) — 1 observations
- **Gate:goose3-6d5** (Gate) — 1 observations
- **Gate:goose3-70e** (Gate) — 1 observations
- **Gate:goose3-684** (Gate) — 2 observations
- **Gate:goose3-4e4** (Gate) — 2 observations
- **Gate:goose3-4pv** (Gate) — 2 observations
- **Gate:goose3-78x** (Gate) — 2 observations
- **Gate:goose3-3p2** (Gate) — 2 observations
- **Gate:goose3-bel** (Gate) — 1 observations
- **Gate:goose3-0e1** (Gate) — 1 observations
- **Interface:GooseACPHandler:on_cancel** (Interface) — 3 observations
- **Interface:GooseACPHandler:on_initialize** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_new_session** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_load_session** (Interface) — 4 observations
- **Interface:GooseACPHandler:on_prompt** (Interface) — 4 observations
- **Interface:GooseAcpSession** (Interface) — 4 observations
- **Interface:GooseAcpAgent** (Interface) — 4 observations
- **Interface:GooseAcpSessionsStore** (Interface) — 3 observations
- **Interface:cli:goose** (Interface) — 1 observations
- **Interface:http:goosed** (Interface) — 3 observations
- **Interface:library:goose** (Interface) — 2 observations
- **Interface:library:goose-mcp** (Interface) — 1 observations
- **goose3-c4w** (Issue) — 7 observations
- **goose core: agent package mode builder API** (PlannedAPI) — 2 observations
- **GooseProject** (Project) — 11 observations
- **Repo:goose3** (Repo) — 2 observations
- **Repo: /home/jmercier/codes/goose3** (RepoPath) — 4 observations
- **Repo: goose3** (RepoPath) — 4 observations
- **RepoPath: crates/goose** (RepoPath) — 4 observations
- **RepoPath: crates/goose-acp** (RepoPath) — 4 observations
- **RepoPath: crates/goose-bench** (RepoPath) — 2 observations
- **RepoPath: crates/goose-cli** (RepoPath) — 4 observations
- **RepoPath: crates/goose-mcp** (RepoPath) — 4 observations
- **RepoPath: crates/goose-server** (RepoPath) — 4 observations
- **RepoPath: crates/goose-test** (RepoPath) — 3 observations
- **RepoPath: crates/goose** (RepoPath) — 1 observations
- **RepoPath: crates/goose-acp** (RepoPath) — 1 observations
- **RepoPath: crates/goose-bench** (RepoPath) — 1 observations
- **RepoPath: crates/goose-cli** (RepoPath) — 1 observations
- **RepoPath: crates/goose-mcp** (RepoPath) — 1 observations
- **RepoPath: crates/goose-server** (RepoPath) — 1 observations
- **RepoPath: crates/goose-test** (RepoPath) — 1 observations
- **RepoPath: crates/goose/src/agents/mod.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose/src/agents/agent.rs** (RepoPath) — 5 observations
- **RepoPath: crates/goose/src/session/extension_data.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose/src/agent_packages/mod.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose/src/agent_packages/builder.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-cli/src/main.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-cli/src/session/mod.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-cli/src/session/builder.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-server/src/main.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-mcp/src/developer/mod.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-mcp/src/recipes/mod.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose/src/agents/subagent_tool.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose/src/agents/subagent_handler.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose/src/session/session_manager.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose/src/recipe/mod.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose/src/recipe/template_recipe.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose/src/scheduler.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-cli/src/session/input.rs** (RepoPath) — 4 observations
- **RepoPath: crates/goose-cli/src/cli.rs** (RepoPath) — 7 observations
- **RepoPath: crates/goose-acp/src/server.rs** (RepoPath) — 9 observations
- **RepoPath: crates/goose-acp/src/transport.rs** (RepoPath) — 8 observations
- **RepoPath: crates/goose-acp/src/transport/http.rs** (RepoPath) — 7 observations
- **RepoPath: crates/goose-acp/src/transport/websocket.rs** (RepoPath) — 7 observations
- **RepoPath: crates/goose/src/slash_commands.rs** (RepoPath) — 1 observations
- **RepoPath: crates/goose-acp/Cargo.toml** (RepoPath) — 2 observations
- **RepoPath:crates/goose-acp/src/server.rs** (RepoPath) — 28 observations
- **RepoPath:crates/goose/src/agents/extension_manager.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose/src/config/extensions.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose/src/agents/extension.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose/src/agent_packages/mod.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose/tests/agent_packages_test.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose-cli/src/cli.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose-server/src/routes/mod.rs** (RepoPath) — 1 observations
- **RepoPath:crates/goose-server/src/openapi.rs** (RepoPath) — 1 observations
- **RepoPath:/tmp/plan-goose3-v6f-acp-mapping-schema.md** (RepoPath) — 1 observations
- **RepoPath:/tmp/plan-goose3-h91-agent-packages-mode-authoring-workflow.md** (RepoPath) — 1 observations
- **RepoPath_GooseACP** (RepoPath) — 5 observations
- **GooseCode_ExtensionConfig** (RepoPath) — 8 observations
- **GooseCode_GooseMode** (RepoPath) — 6 observations
- **GooseCode_SummonExtension** (RepoPath) — 10 observations
- **GooseCode_Recipe** (RepoPath) — 9 observations
- **GooseCode_ACP** (RepoPath) — 10 observations
- **GooseCode_Registry** (RepoPath) — 13 observations
- **GooseCode_CLI_Registry** (RepoPath) — 7 observations
- **GooseCode_Server_Registry** (RepoPath) — 4 observations
- **goose_acp_server_rs** (RepoPath) — 4 observations
- **goose_registry_mod_rs** (RepoPath) — 3 observations
- **goose_server_registry_routes** (RepoPath) — 3 observations
- **repopath:goose-arch-extension-agent-separation** (RepoPath) — 4 observations
- **repopath:goose-arch-unified-acp-api** (RepoPath) — 4 observations
- **repopath:goose-design-meta-orchestrator** (RepoPath) — 4 observations
- **repopath:goose-design-multi-agent** (RepoPath) — 4 observations
- **repopath:goose-design-multi-layer** (RepoPath) — 4 observations
- **repopath:goose-protocol-disambiguation** (RepoPath) — 4 observations
- **RepoPath:/home/jmercier/codes/goose4** (RepoPath) — 2 observations
- **Goose_A2A_Compat_Module** (RepoPath) — 9 observations
- **RepoProfile:goose3** (RepoProfile) — 14 observations
- **Risk-GooseAgentToolVisibility** (Risk) — 6 observations
- **Source: snippet goose-cli cli.rs 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: snippet goose-acp server.rs 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: snippet goose-acp transport.rs 2026-02-06T20:40:00** (Source) — 1 observations
- **Source: snippet goose-cli cli.rs 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: snippet goose-acp server.rs 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: snippet goose slash_commands.rs 2026-02-06T20:42:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:38:00** (Source) — 1 observations
- **Source: goose-acp uses agent-client-protocol 2026-02-06T22:38:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:37:00** (Source) — 1 observations
- **Source: goose-acp mentions agent-client-protocol 2026-02-06T22:37:00** (Source) — 1 observations
- **Source: goose-acp Cargo.toml 2026-02-06T22:44:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T22:44:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp Cargo.toml 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp server.rs head 2026-02-06T23:05:00** (Source) — 1 observations
- **Source: goose-acp uses sacp 2026-02-06T23:40:00** (Source) — 1 observations
- **Source:artifact:/tmp/research-goose3-c33-agent-packages.md** (Source) — 2 observations
- **Source:/tmp/plan-goose3-om3-agent-packages-modes.md** (Source) — 3 observations
- **Source:/tmp/verify-goose3-5mp-agent-packages.md** (Source) — 1 observations
- **Source:/tmp/research-goose3-3ho-cli-server-agent-packages.md** (Source) — 1 observations
- **Source:/tmp/plan-goose3-9af-cli-server-agent-packages.md** (Source) — 4 observations
- **Source:local:/tmp/research-goose3-695-agent-packages-mode-authoring.md** (Source) — 2 observations
- **Source:local:/tmp/research-goose3-5ik-sacp-to-agent-client-protocol.md** (Source) — 2 observations
- **Source:local:/tmp/plan-goose3-bjg-sacp-to-agent-client-protocol.md** (Source) — 2 observations
- **GooseMultiAgentArchDoc** (Source) — 4 observations
- **Symbol:RepetitionInspector@crates/goose/src/tool_monitor.rs:34** (Symbol) — 1 observations
- **Symbol:InspectionResult@crates/goose/src/tool_inspection.rs:12** (Symbol) — 1 observations
- **Symbol:InspectionAction@crates/goose/src/tool_inspection.rs:23** (Symbol) — 1 observations
- **Symbol:ToolInspector@crates/goose/src/tool_inspection.rs:34** (Symbol) — 1 observations
- **Symbol:ToolInspectionManager@crates/goose/src/tool_inspection.rs:56** (Symbol) — 1 observations
- **Symbol:apply_inspection_results_to_permissions@crates/goose/src/tool_inspection.rs:175** (Symbol) — 1 observations
- **Symbol:get_security_finding_id_from_results@crates/goose/src/tool_inspection.rs:257** (Symbol) — 1 observations
- **Symbol:RetryConfig@crates/goose/src/providers/retry.rs:14** (Symbol) — 1 observations
- **Symbol:should_retry@crates/goose/src/providers/retry.rs:74** (Symbol) — 1 observations
- **Symbol:ProviderRetry@crates/goose/src/providers/retry.rs:126** (Symbol) — 1 observations
- **Symbol:XaiProvider@crates/goose/src/providers/xai.rs:44** (Symbol) — 1 observations
- **Symbol:TetrateProvider@crates/goose/src/providers/tetrate.rs:35** (Symbol) — 1 observations
- **Symbol:LeadWorkerProvider@crates/goose/src/providers/lead_worker.rs:16** (Symbol) — 1 observations
- **Symbol:action_required_manager@crates/goose/src/lib.rs:1** (Symbol) — 1 observations
- **Symbol:agent_packages@crates/goose/src/lib.rs:2** (Symbol) — 1 observations
- **Symbol:agents@crates/goose/src/lib.rs:3** (Symbol) — 1 observations
- **Symbol:config@crates/goose/src/lib.rs:4** (Symbol) — 1 observations
- **Symbol:context_mgmt@crates/goose/src/lib.rs:5** (Symbol) — 1 observations
- **Symbol:conversation@crates/goose/src/lib.rs:6** (Symbol) — 1 observations
- **Symbol:execution@crates/goose/src/lib.rs:7** (Symbol) — 1 observations
- **Symbol:goose_apps@crates/goose/src/lib.rs:8** (Symbol) — 1 observations
- **Symbol:hints@crates/goose/src/lib.rs:9** (Symbol) — 1 observations
- **Symbol:logging@crates/goose/src/lib.rs:10** (Symbol) — 1 observations
- **Symbol:mcp_utils@crates/goose/src/lib.rs:11** (Symbol) — 1 observations
- **Symbol:model@crates/goose/src/lib.rs:12** (Symbol) — 1 observations
- **Symbol:oauth@crates/goose/src/lib.rs:13** (Symbol) — 1 observations
- **Symbol:permission@crates/goose/src/lib.rs:14** (Symbol) — 1 observations
- **Symbol:posthog@crates/goose/src/lib.rs:15** (Symbol) — 1 observations
- **Symbol:prompt_template@crates/goose/src/lib.rs:16** (Symbol) — 1 observations
- **Symbol:providers@crates/goose/src/lib.rs:17** (Symbol) — 1 observations
- **Symbol:shutdown_signal@crates/goose-cli/src/signal.rs:6** (Symbol) — 1 observations
- **Symbol:shutdown_signal@crates/goose-cli/src/signal.rs:30** (Symbol) — 1 observations
- **Symbol:cli@crates/goose-cli/src/lib.rs:1** (Symbol) — 1 observations
- **Symbol:commands@crates/goose-cli/src/lib.rs:2** (Symbol) — 1 observations
- **Symbol:logging@crates/goose-cli/src/lib.rs:3** (Symbol) — 1 observations
- **Symbol:project_tracker@crates/goose-cli/src/lib.rs:4** (Symbol) — 1 observations
- **Symbol:recipes@crates/goose-cli/src/lib.rs:5** (Symbol) — 1 observations
- **Symbol:scenario_tests@crates/goose-cli/src/lib.rs:6** (Symbol) — 1 observations
- **Symbol:session@crates/goose-cli/src/lib.rs:7** (Symbol) — 1 observations
- **Symbol:signal@crates/goose-cli/src/lib.rs:8** (Symbol) — 1 observations
- **Symbol:ScenarioResult@crates/goose-cli/src/scenario_tests/scenario_runner.rs:25** (Symbol) — 1 observations
- **Symbol:ProviderConfig@crates/goose-cli/src/scenario_tests/provider_configs.rs:8** (Symbol) — 1 observations
- **Symbol:get_provider_configs@crates/goose-cli/src/scenario_tests/provider_configs.rs:103** (Symbol) — 1 observations
- **Symbol:AgentPackagesCommand@crates/goose-cli/src/cli.rs:50** (Symbol) — 1 observations
- **Symbol:InitAgentPackageArgs@crates/goose-cli/src/cli.rs:68** (Symbol) — 1 observations
- **Symbol:CreateModeArgs@crates/goose-cli/src/cli.rs:83** (Symbol) — 1 observations
- **Symbol:Identifier@crates/goose-cli/src/cli.rs:107** (Symbol) — 1 observations
- **Symbol:SessionOptions@crates/goose-cli/src/cli.rs:138** (Symbol) — 1 observations
- **Symbol:ExtensionOptions@crates/goose-cli/src/cli.rs:165** (Symbol) — 1 observations
- **Symbol:InputOptions@crates/goose-cli/src/cli.rs:196** (Symbol) — 1 observations
- **Symbol:OutputOptions@crates/goose-cli/src/cli.rs:279** (Symbol) — 1 observations
- **Symbol:ModelOptions@crates/goose-cli/src/cli.rs:310** (Symbol) — 1 observations
- **Symbol:RunBehavior@crates/goose-cli/src/cli.rs:332** (Symbol) — 1 observations
- **Symbol:BenchCommand@crates/goose-cli/src/cli.rs:610** (Symbol) — 1 observations
- **Symbol:InputConfig@crates/goose-cli/src/cli.rs:1016** (Symbol) — 1 observations
- **Symbol:text@crates/goose-cli/src/scenario_tests/message_generator.rs:12** (Symbol) — 1 observations
- **Symbol:image@crates/goose-cli/src/scenario_tests/message_generator.rs:17** (Symbol) — 1 observations
- **Symbol:setup_logging@crates/goose-cli/src/logging.rs:24** (Symbol) — 1 observations
- **Symbol:Theme@crates/goose-cli/src/session/output.rs:22** (Symbol) — 1 observations
- **Symbol:set_theme@crates/goose-cli/src/session/output.rs:69** (Symbol) — 1 observations
- **Symbol:auth@crates/goose-server/src/lib.rs:1** (Symbol) — 1 observations
- **Symbol:configuration@crates/goose-server/src/lib.rs:2** (Symbol) — 1 observations
- **Symbol:error@crates/goose-server/src/lib.rs:3** (Symbol) — 1 observations
- **Symbol:openapi@crates/goose-server/src/lib.rs:4** (Symbol) — 1 observations
- **Symbol:routes@crates/goose-server/src/lib.rs:5** (Symbol) — 1 observations
- **Symbol:state@crates/goose-server/src/lib.rs:6** (Symbol) — 1 observations
- **Symbol:tunnel@crates/goose-server/src/lib.rs:7** (Symbol) — 1 observations
- **Symbol:ConfigError@crates/goose-server/src/error.rs:4** (Symbol) — 1 observations
- **Symbol:Settings@crates/goose-server/src/configuration.rs:7** (Symbol) — 1 observations
- **Symbol:action_required@crates/goose-server/src/routes/mod.rs:1** (Symbol) — 1 observations
- **Symbol:agent@crates/goose-server/src/routes/mod.rs:2** (Symbol) — 1 observations
- **Symbol:agent_packages@crates/goose-server/src/routes/mod.rs:3** (Symbol) — 1 observations
- **Symbol:audio@crates/goose-server/src/routes/mod.rs:4** (Symbol) — 1 observations
- **Symbol:config_management@crates/goose-server/src/routes/mod.rs:5** (Symbol) — 1 observations
- **Symbol:errors@crates/goose-server/src/routes/mod.rs:6** (Symbol) — 1 observations
- **Symbol:mcp_app_proxy@crates/goose-server/src/routes/mod.rs:7** (Symbol) — 1 observations
- **Symbol:mcp_ui_proxy@crates/goose-server/src/routes/mod.rs:8** (Symbol) — 1 observations
- **Symbol:prompts@crates/goose-server/src/routes/mod.rs:9** (Symbol) — 1 observations
- **Symbol:recipe@crates/goose-server/src/routes/mod.rs:10** (Symbol) — 1 observations
- **Symbol:recipe_utils@crates/goose-server/src/routes/mod.rs:11** (Symbol) — 1 observations
- **Symbol:reply@crates/goose-server/src/routes/mod.rs:12** (Symbol) — 1 observations
- **Symbol:schedule@crates/goose-server/src/routes/mod.rs:13** (Symbol) — 1 observations
- **Symbol:session@crates/goose-server/src/routes/mod.rs:14** (Symbol) — 1 observations
- **Symbol:setup@crates/goose-server/src/routes/mod.rs:15** (Symbol) — 1 observations
- **Symbol:status@crates/goose-server/src/routes/mod.rs:16** (Symbol) — 1 observations
- **Symbol:telemetry@crates/goose-server/src/routes/mod.rs:17** (Symbol) — 1 observations
- **Symbol:tunnel@crates/goose-server/src/routes/mod.rs:18** (Symbol) — 1 observations
- **Symbol:utils@crates/goose-server/src/routes/mod.rs:19** (Symbol) — 1 observations
- **Symbol:configure@crates/goose-server/src/routes/mod.rs:26** (Symbol) — 1 observations
- **Symbol:AppState@crates/goose-server/src/state.rs:18** (Symbol) — 1 observations
- **Symbol:autovisualiser@crates/goose-mcp/src/lib.rs:12** (Symbol) — 1 observations
- **Symbol:computercontroller@crates/goose-mcp/src/lib.rs:13** (Symbol) — 1 observations
- **Symbol:developer@crates/goose-mcp/src/lib.rs:14** (Symbol) — 1 observations
- **Symbol:mcp_server_runner@crates/goose-mcp/src/lib.rs:15** (Symbol) — 1 observations
- **Symbol:tutorial@crates/goose-mcp/src/lib.rs:17** (Symbol) — 1 observations
- **Symbol:BuiltinDef@crates/goose-mcp/src/lib.rs:27** (Symbol) — 1 observations
- **Symbol:analyze@crates/goose-mcp/src/developer/mod.rs:1** (Symbol) — 1 observations
- **Symbol:paths@crates/goose-mcp/src/developer/mod.rs:4** (Symbol) — 1 observations
- **Symbol:rmcp_developer@crates/goose-mcp/src/developer/mod.rs:8** (Symbol) — 1 observations
- **Symbol:LoadTutorialParams@crates/goose-mcp/src/tutorial/mod.rs:18** (Symbol) — 1 observations
- **Symbol:TutorialServer@crates/goose-mcp/src/tutorial/mod.rs:25** (Symbol) — 1 observations
- **Symbol:get_language_identifier@crates/goose-mcp/src/developer/lang.rs:4** (Symbol) — 1 observations
- **Symbol:ShellConfig@crates/goose-mcp/src/developer/shell.rs:8** (Symbol) — 1 observations
- **Symbol:expand_path@crates/goose-mcp/src/developer/shell.rs:70** (Symbol) — 1 observations
- **Symbol:is_absolute_path@crates/goose-mcp/src/developer/shell.rs:85** (Symbol) — 1 observations
- **Symbol:normalize_line_endings@crates/goose-mcp/src/developer/shell.rs:95** (Symbol) — 1 observations
- **Symbol:configure_shell_command@crates/goose-mcp/src/developer/shell.rs:109** (Symbol) — 1 observations
- **Symbol:RelaceEditor@crates/goose-mcp/src/developer/editor_models/relace_editor.rs:8** (Symbol) — 1 observations
- **Symbol:McpCommand@crates/goose-mcp/src/mcp_server_runner.rs:7** (Symbol) — 1 observations
- **Symbol:OpenAICompatibleEditor@crates/goose-mcp/src/developer/editor_models/openai_compatible_editor.rs:8** (Symbol) — 1 observations
- **Symbol:DiffResults@crates/goose-mcp/src/developer/text_editor.rs:98** (Symbol) — 1 observations
- **Symbol:calculate_view_range@crates/goose-mcp/src/developer/text_editor.rs:424** (Symbol) — 1 observations
- **Symbol:format_file_content@crates/goose-mcp/src/developer/text_editor.rs:466** (Symbol) — 1 observations
- **Symbol:recommend_read_range@crates/goose-mcp/src/developer/text_editor.rs:513** (Symbol) — 1 observations
- **Symbol:save_file_history@crates/goose-mcp/src/developer/text_editor.rs:1086** (Symbol) — 1 observations
- **Symbol:MorphLLMEditor@crates/goose-mcp/src/developer/editor_models/morphllm_editor.rs:8** (Symbol) — 1 observations
- **Symbol:EditorModel@crates/goose-mcp/src/developer/editor_models/mod.rs:13** (Symbol) — 1 observations
- **Symbol:EditorModelImpl@crates/goose-mcp/src/developer/editor_models/mod.rs:57** (Symbol) — 1 observations
- **Symbol:create_editor_model@crates/goose-mcp/src/developer/editor_models/mod.rs:71** (Symbol) — 1 observations
- **Symbol:SaveAsFormat@crates/goose-mcp/src/computercontroller/mod.rs:32** (Symbol) — 1 observations
- **Symbol:Provider@crates/goose/src/providers/base.rs:360** (Symbol) — 1 observations
- **Symbol:Agent@crates/goose/src/agents/agent.rs:114** (Symbol) — 1 observations
- **Symbol:Session@crates/goose/src/session/session_manager.rs:67** (Symbol) — 1 observations
- **Symbol:MemoryServer@crates/goose-mcp/src/memory/mod.rs:65** (Symbol) — 1 observations
- **Symbol:DeveloperServer@crates/goose-mcp/src/developer/rmcp_developer.rs:175** (Symbol) — 1 observations
- **Symbol:ComputerControllerServer@crates/goose-mcp/src/computercontroller/mod.rs:280** (Symbol) — 1 observations
- **Task: goose2-uv9** (Task) — 7 observations
- **TestEvidence: Verify phase completed for goose3-daa @ 2026-02-06T21:30:00** (TestEvidence) — 1 observations
- **TestEvidence: Closed ready gate goose3-8ac @ 2026-02-06T22:07:00** (TestEvidence) — 1 observations
- **TestEvidence: Closed verify goose3-2w8 and gate goose3-uvv @ 2026-02-06T22:22:00** (TestEvidence) — 1 observations
- **TestEvidence:goose3-5mp:agent_packages** (TestEvidence) — 5 observations
- **TestEvidence:goose3-6m3:e57973627782dd693c13f6ac416587d8db53ab01

e57973627782dd693c13f6ac416587d8db53ab01** (TestEvidence) — 3 observations
- **TestEvidence:goose3-4jw:9212f3632029a7617c73fb77758c56fab0237c97

9212f3632029a7617c73fb77758c56fab0237c97** (TestEvidence) — 2 observations
- **TestEvidence:goose3-e6n:62ca710a36896fef6fccdd8546a0c79d9bdb9b07

62ca710a36896fef6fccdd8546a0c79d9bdb9b07** (TestEvidence) — 4 observations
- **TestEvidence:goose3-puw:kg-lookups** (TestEvidence) — 10 observations
- **TestEvidence:goose3-2rz:a38f5c900** (TestEvidence) — 5 observations
- **TestEvidence:goose3-g3n:a38f5c900** (TestEvidence) — 4 observations
- **TestEvidence:arch-scan:goose-and-server** (TestEvidence) — 12 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint:2026-02-08T00:22:00** (TestEvidence) — 17 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint-fix:2026-02-08T00:26:00** (TestEvidence) — 12 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-endpoint-verified:2026-02-08T00:32:00** (TestEvidence) — 6 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:53:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:54:00** (TestEvidence) — 15 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test:2026-02-08T00:57:00** (TestEvidence) — 13 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-test-verified:2026-02-08T00:57:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-initialize-smoke-verified:2026-02-08T01:02:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T01:42:00** (TestEvidence) — 18 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:00:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:impl:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:20:00** (TestEvidence) — 5 observations
- **TestEvidence:acp-roadmap:verify:goosed-ws-newsession-prompt-smoke-verified:2026-02-08T02:48:00** (TestEvidence) — 6 observations
- **GooseAgent-ToolGroups-Tests** (TestEvidence) — 4 observations
- **TestSurface:goose:tests** (TestSurface) — 1 observations
- **TestSurface:goose-cli:tests** (TestSurface) — 1 observations
- **TestSurface:goose-server:tests** (TestSurface) — 1 observations
- **TestSurface:goose-mcp:tests** (TestSurface) — 1 observations
- **Goal: goose session parity with goose-acp-server agent packages/modes** (WorkItem) — 15 observations
- **goose3-9uc** (task) — 15 observations
- **goose3-au7** (task) — 6 observations
- **goose3-ls4** (task) — 56 observations
- **goose3-owz** (task) — 23 observations
- **goose3-1g1** (task) — 17 observations
- **goose3-8ba** (task) — 4 observations
- **goose3-pni** (task) — 3 observations
- **goose3-1g1-agent-package-cli** (workstream) — 43 observations

### Agent Registry (38 entities)

- **BeadsEpic_AgentRegistry** (BeadsEpic) — 6 observations
- **BeadsEpic-AgentRegistry** (BeadsEpic) — 5 observations
- **goose-acp: ModeRegistry** (CodeAnchor) — 1 observations
- **AgentRegistry_ARegistryAI** (Component) — 16 observations
- **Glama_MCP_Registry** (Component) — 8 observations
- **Smithery_MCP_Registry** (Component) — 6 observations
- **Aregistry_AI** (Component) — 12 observations
- **RegistryManager** (Component) — 13 observations
- **RegistryModule** (Component) — 8 observations
- **AgentSlotRegistry** (Component) — 5 observations
- **AgentSlotRegistryServer** (Component) — 6 observations
- **AgentRegistry_Concept** (Concept) — 4 observations
- **MCP_Ecosystem_Registry** (Concept) — 7 observations
- **AgentRegistry_Patterns** (Concept) — 6 observations
- **AgentRegistry-Concept** (Concept) — 9 observations
- **RegistrySystem** (Concept) — 8 observations
- **AI_Agent_Registry** (Concept) — 9 observations
- **AgentRegistry_GooseRelevance** (Decision) — 10 observations
- **AgentRegistry_GooseRecommendations_V2** (Decision) — 10 observations
- **Decision_AgentRegistryArchitecture** (Decision) — 27 observations
- **Decision_DropWorkflowRegistry** (Decision) — 5 observations
- **AgentRegistry_Landscape_Analysis** (Finding) — 10 observations
- **AgentRegistry_Manifest_Comparison** (Finding) — 6 observations
- **AgentRegistry_Packaging_Comparison** (Finding) — 7 observations
- **AgentRegistry_Landscape_V2** (Finding) — 11 observations
- **Finding_RegistryTaxonomy_5Types** (Finding) — 48 observations
- **Finding_MCPOfficialRegistry** (Finding) — 7 observations
- **Finding_ACPClient_Registry_Schema** (Finding) — 9 observations
- **WorkSummary-AgentRegistry-Feature** (Finding) — 15 observations
- **CodeReview_AgentRegistry_Branch** (Finding) — 10 observations
- **RepoPath_ProviderRegistry** (RepoPath) — 3 observations
- **GooseCode_Registry** (RepoPath) — 13 observations
- **GooseCode_CLI_Registry** (RepoPath) — 7 observations
- **GooseCode_Server_Registry** (RepoPath) — 4 observations
- **goose_registry_mod_rs** (RepoPath) — 3 observations
- **goose_server_registry_routes** (RepoPath) — 3 observations
- **Source_Aregistry** (Source) — 4 observations
- **TestEvidence:agent-registry:index+mode-metadata:verified:3dca8eeacf7d28bb164674b19c55b4c6d322d83c** (TestEvidence) — 6 observations

### Beads Tasks (95 entities)

- **BeadsEpic: goose3-dcq** (BeadsEpic) — 2 observations
- **BeadsEpic: goose3-daa** (BeadsEpic) — 2 observations
- **BeadsEpic: goose3-hgx** (BeadsEpic) — 5 observations
- **BeadsEpic:goose3-9l5** (BeadsEpic) — 3 observations
- **BeadsEpic:goose3-zw6** (BeadsEpic) — 3 observations
- **BeadsEpic:goose3-c1q** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-ts8** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-mly** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-tuh** (BeadsEpic) — 2 observations
- **BeadsEpic:goose3-vjv** (BeadsEpic) — 1 observations
- **goose3-vjv** (BeadsEpic) — 4 observations
- **goose3-xts** (BeadsEpic) — 5 observations
- **goose3-70l** (BeadsEpic) — 5 observations
- **goose3-wlz** (BeadsEpic) — 45 observations
- **BeadsEpic:goose3-wlz** (BeadsEpic) — 2 observations
- **BeadsEpic_AgentRegistry** (BeadsEpic) — 6 observations
- **Goose-MetaOrchestrator** (BeadsEpic) — 9 observations
- **BeadsEpic-AgentRegistry** (BeadsEpic) — 5 observations
- **BeadsEpic-AgentModes** (BeadsEpic) — 4 observations
- **BeadsEpic-MetaOrchestrator** (BeadsEpic) — 4 observations
- **BeadsEpic-E2E-Testing** (BeadsEpic) — 5 observations
- **BeadsTask: goose3-61d** (BeadsTask) — 2 observations
- **BeadsTask: goose3-vr0** (BeadsTask) — 2 observations
- **BeadsTask: goose3-mf0** (BeadsTask) — 2 observations
- **BeadsTask: goose3-dhr** (BeadsTask) — 2 observations
- **BeadsTask: goose3-3i0** (BeadsTask) — 1 observations
- **BeadsTask: goose3-2w8** (BeadsTask) — 1 observations
- **BeadsTask: goose3-tqp** (BeadsTask) — 2 observations
- **BeadsTask: goose3-fyj** (BeadsTask) — 2 observations
- **BeadsTask:goose3-c33** (BeadsTask) — 1 observations
- **BeadsTask:goose3-om3** (BeadsTask) — 1 observations
- **BeadsTask:goose3-1iz** (BeadsTask) — 1 observations
- **BeadsTask:goose3-5mp** (BeadsTask) — 1 observations
- **BeadsTask:goose3-3ho** (BeadsTask) — 2 observations
- **BeadsTask:goose3-9af** (BeadsTask) — 2 observations
- **BeadsTask:goose3-6m3** (BeadsTask) — 2 observations
- **BeadsTask:goose3-4jw** (BeadsTask) — 3 observations
- **BeadsTask:goose3-fyj** (BeadsTask) — 2 observations
- **BeadsTask:goose3-kkt** (BeadsTask) — 2 observations
- **BeadsTask:goose3-q2g** (BeadsTask) — 2 observations
- **BeadsTask:goose3-c2v** (BeadsTask) — 2 observations
- **BeadsTask:goose3-2qy** (BeadsTask) — 2 observations
- **BeadsTask:goose3-5na** (BeadsTask) — 2 observations
- **BeadsTask:goose3-5ik** (BeadsTask) — 2 observations
- **BeadsTask:goose3-bjg** (BeadsTask) — 2 observations
- **goose3-2rz** (BeadsTask) — 63 observations
- **goose3-g3n** (BeadsTask) — 5 observations
- **goose3-35e** (BeadsTask) — 5 observations
- **goose3-4iu** (BeadsTask) — 2 observations
- **goose3-95o** (BeadsTask) — 2 observations
- **goose3-h6u** (BeadsTask) — 2 observations
- **goose3-8v1** (BeadsTask) — 7 observations
- **goose3-vvf** (BeadsTask) — 10 observations
- **goose3-eyb** (BeadsTask) — 7 observations
- **goose3-zpg** (BeadsTask) — 5 observations
- **goose3-2b0** (BeadsTask) — 3 observations
- **BeadsTask:goose3-eyb** (BeadsTask) — 4 observations
- **BeadsTask:goose3-zpg** (BeadsTask) — 1 observations
- **BeadsTask:goose3-2b0** (BeadsTask) — 1 observations
- **BeadsTask:goose3-350** (BeadsTask) — 6 observations
- **BeadsTask:goose3-8cg** (BeadsTask) — 6 observations
- **BeadsTask:goose3-9uc** (BeadsTask) — 55 observations
- **BeadsTask:goose3-c4w** (BeadsTask) — 15 observations
- **BeadsTask:goose3-8ba** (BeadsTask) — 3 observations
- **BeadsTask:goose3-ooa** (BeadsTask) — 3 observations
- **BeadsTask:goose3-rq7** (BeadsTask) — 3 observations
- **BeadsTask_P1_Manifest** (BeadsTask) — 3 observations
- **BeadsTask_P1_Trait** (BeadsTask) — 3 observations
- **BeadsTask_P1_Local** (BeadsTask) — 3 observations
- **BeadsTask_P2_GitHub** (BeadsTask) — 3 observations
- **BeadsTask_P2_HTTP** (BeadsTask) — 3 observations
- **BeadsTask_P3_CLI** (BeadsTask) — 3 observations
- **BeadsTask_P4_Server** (BeadsTask) — 3 observations
- **BeadsTask_P5_Publish** (BeadsTask) — 3 observations
- **BeadsTask_P6_Integrate** (BeadsTask) — 3 observations
- **Gate: goose3-aau** (Gate) — 3 observations
- **Gate: goose3-60j** (Gate) — 2 observations
- **Gate: goose3-j8k** (Gate) — 2 observations
- **Gate: goose3-xzf** (Gate) — 2 observations
- **Gate: goose3-w7u** (Gate) — 1 observations
- **Gate: goose3-8ac** (Gate) — 1 observations
- **Gate: goose3-uvv** (Gate) — 1 observations
- **Gate:goose3-p6l** (Gate) — 1 observations
- **Gate:goose3-du1** (Gate) — 1 observations
- **Gate:goose3-6d5** (Gate) — 1 observations
- **Gate:goose3-70e** (Gate) — 1 observations
- **Gate:goose3-684** (Gate) — 2 observations
- **Gate:goose3-4e4** (Gate) — 2 observations
- **Gate:goose3-4pv** (Gate) — 2 observations
- **Gate:goose3-78x** (Gate) — 2 observations
- **Gate:goose3-3p2** (Gate) — 2 observations
- **Gate:goose3-bel** (Gate) — 1 observations
- **Gate:goose3-0e1** (Gate) — 1 observations
- **BeadsTask_PlanGate** (Gate) — 4 observations
- **EpicCompletion_AgentPersonaCleanup** (Gate) — 5 observations

## Reload Instructions

### From Goose

Ask goose:
```
Reload the knowledge graph from docs/kg/graph.json
```

### Programmatic

```bash
python3 docs/kg/reload.py --stats-only  # verify
python3 docs/kg/reload.py --emit-calls > /tmp/kg-calls.json  # generate MCP calls
```

### File Inventory

| File | Purpose |
|------|---------|
| `graph.json` | Complete graph (entities + relations), canonical lossless export |
| `entities.json` | Entities only, for selective reload |
| `relations.json` | Relations only, for selective reload |
| `manifest.json` | Export metadata: counts, types, timestamp |
| `ontology.md` | This file — human-readable schema + domain overview |
| `reload.py` | Script to reload graph into MCP memory server |
