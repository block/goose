# Knowledge Graph Ontology

**Entities**: 1566 | **Relations**: 2222
**Updated**: 2026-02-17T21:56:00Z | **Branch**: feature/cli-via-goosed

## Entity Types (56)

| Type | Count |
|------|-------|
| Interface | 488 |
| Component | 147 |
| Source | 137 |
| Finding | 126 |
| Symbol | 126 |
| RepoPath | 96 |
| Concept | 86 |
| Decision | 65 |
| BeadsTask | 55 |
| CodeArtifact | 46 |
| TestEvidence | 45 |
| BeadsEpic | 23 |
| Gate | 20 |
| Risk | 12 |
| DependencyEdge | 9 |
| Feature | 8 |
| task | 7 |
| Phase | 6 |
| CodeAnchor | 5 |
| ExecutionFlow | 5 |
| CodeComponent | 5 |
| TestSurface | 4 |
| Specification | 4 |
| Library | 3 |
| WorkItem | 2 |
| Design | 2 |
| Spec | 2 |
| Organization | 2 |
| UIComponent | 2 |
| APIEndpoints | 2 |
| Constraint | 1 |
| PlannedAPI | 1 |
| Task | 1 |
| Epic | 1 |
| Plan | 1 |
| RepoProfile | 1 |
| ArchitectureDecision | 1 |
| API | 1 |
| Repo | 1 |
| Issue | 1 |
| workstream | 1 |
| component | 1 |
| Config | 1 |
| Test | 1 |
| Project | 1 |
| Protocol | 1 |
| Crate | 1 |
| DocumentationSite | 1 |
| Claim | 1 |
| Repository | 1 |
| Application | 1 |
| Architecture | 1 |
| BackendComponent | 1 |
| ArchitectureAnalysis | 1 |
| DesignPattern | 1 |
| DesignConcept | 1 |

## Relation Types (113)

| Type | Count |
|------|-------|
| related_to | 632 |
| documents | 222 |
| defines | 165 |
| derived_from | 141 |
| implements | 129 |
| defined_in | 126 |
| depends_on | 98 |
| located_at | 71 |
| uses | 69 |
| validated_by | 52 |
| owns | 46 |
| affects | 44 |
| documented_by | 43 |
| implemented_by | 28 |
| belongs_to | 24 |
| contains | 21 |
| verifies | 20 |
| compares | 17 |
| blocks | 16 |
| has_evidence | 14 |
| extends | 14 |
| informs | 13 |
| touches | 12 |
| owned_by | 11 |
| supports | 8 |
| describes | 7 |
| exposes | 7 |
| has_child | 7 |
| routes_to | 7 |
| enables | 6 |
| references | 6 |
| modifies | 6 |
| handles | 5 |
| tracks | 5 |
| requires | 4 |
| returns | 4 |
| includes | 4 |
| integrates_with | 4 |
| will_contain | 4 |
| analyzes | 4 |
| constrains | 3 |
| unblocks | 3 |
| implemented_in | 3 |
| used_by | 3 |
| exposed_via | 3 |
| is published in | 3 |
| part_of | 3 |
| is replaced by | 2 |
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
| configures | 2 |
| reviews_implementation_of | 2 |
| specifies | 2 |
| can refer to | 2 |
| is disambiguated by | 2 |
| renders_inline | 2 |
| coordinates | 2 |
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
| can_integrate_with | 1 |
| target_of | 1 |
| analyzed_in | 1 |
| builds_on | 1 |
| mitigates | 1 |
| mirrors | 1 |
| will_use | 1 |
| reads_from | 1 |
| writes_to | 1 |
| violates | 1 |

## Key Entity Clusters

### A2A Protocol & Spec Compliance
- ProtoJSON_ADR001, LenientDeserialization, A2AJsSdkNonCompliance
- A2A Protocol, A2AClient, A2A Server, AgentCard
- All 9 error codes (-32001 to -32009)

### Multi-Agent Infrastructure
- DispatcherTrait (InProcess, A2A, Composite)
- CompoundExecutionFanout (sequential sub-task dispatch)
- SlotDelegation (InProcess, ExternalAcp, RemoteA2A)
- DynamicSlotSync (registry → orchestrator sync)
- PerAgentExtensionScoping (fully wired end-to-end)

### Server Endpoints
- AgentCatalogEndpoint (GET /agents/catalog)
- ObservatoryEndpoints (dashboard, active-agents, health)

### Proposals & Risks
- DualIdentityProposal (agent+user dual ID system)
- PipelineModuleWIP (frontend agent, build-breaking risk)

### Design Documents
- docs/design/goose-multi-agent-architecture.md
- docs/design/meta-orchestrator-architecture.md
- docs/design/ux-redesign-proposal.md
- docs/roadmap/goose-multi-agent-roadmap.markmap.md

### Research Artifacts (tmp/docs/)
- research-a2a-interop-gaps.mdx
- research-server-route-gaps.mdx
- a2a-spec-compliance-report.mdx
- plan-backend-dispatcher-routes.mdx
- proposal-dual-identity-agent-user.mdx
- session-summary-2026-02-17-backend-a2a.mdx

## Graph Files

| File | Size |
|------|------|
| entities.json | 2,816,401 bytes |
| relations.json | 307,512 bytes |
| graph.json | 3,142,552 bytes |
| graph.json.gz | 645,228 bytes |
| manifest.json | 947 bytes |
