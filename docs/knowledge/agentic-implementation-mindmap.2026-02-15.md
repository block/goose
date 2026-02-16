# Agentic implementation mind map (2026-02-15)

Local-only, retrieval-first mind map for future **code understanding**, **audits**, **debugging**, and **research/SOTA** work.

- KG seed (JSONL, for later import into Knowledge Graph Memory):
  - `docs/knowledge/kg-seeds/agentic-implementation-mindmap.2026-02-15.jsonl`
- Protocol disambiguation note (tracked):
  - `docs/knowledge/protocols-acp-vs-a2a-disambiguation.md`

## Mind map

```mermaid
mindmap
  root((Agentic implementation knowledge (2026)))

    ProtocolTaxonomy[Protocol taxonomy / acronym collisions]
      A2A[A2A = Agent2Agent Protocol]
        Concepts
          AgentCard[Agent Card (capabilities)]
          Task[Task lifecycle (incl long-running)]
          Artifact[Artifacts (task outputs)]
          Transport[JSON-RPC 2.0 over HTTP(S)]
      BeeAI_ACP[ACP = Agent Communication Protocol (BeeAI/IBM ecosystem)]
        Evidence
          OpenAPI[OpenAPI spec: title 'ACP - Agent Communication Protocol']
          ACPDocs[ACP docs page: MCP and A2A]
      ACP_IDE[ACP = Agent Client Protocol (editor ↔ coding agent)]
        Evidence
          DocsRS[docs.rs crate: agent-client-protocol]

    GooseArchitecture[Goose architecture touchpoints]
      Server[goose-server (goosed)]
        Routes[ACP-superset routes / sessions / runs / registry]
      Core[goose crate]
        Agents[agent/orchestrator logic]
        Registry[registry sources (local/GitHub/HTTP/A2A)]
      CLI[goose-cli]
        Client[GoosedClient + streaming]
      Desktop[ui/desktop]
        OpenAPI[OpenAPI-generated TS client]

    RetrievalHooks[Retrieval hooks for audits/debugging]
      AvoidAcronymCollision[Always disambiguate "ACP" (Agent Communication vs Agent Client)]
      PreferPrimarySources[Prefer primary sources + pinned repo paths]
      EvidenceArtifacts[Keep evidence artifacts (HTML/YAML) + link them]
      EnvSpecificFindings[Mark environment-specific findings (e.g., crates.io HTML vs API)]

    RepoDocs[Repo docs (high-signal entry points)]
      UnifiedACP[docs/architecture/unified-acp-api.md]
      ExtAgentSep[docs/architecture/extension-agent-separation.md]
      MetaOrch[docs/design/meta-orchestrator-architecture.md]
      MultiLayer[docs/design/multi-layer-orchestrator.md]
      MultiAgent[docs/design/goose-multi-agent-architecture.md]
      Reviews[docs/reviews/*]
      Roadmap[docs/roadmap/*]
```

## Primary external sources (URLs)

These are the main authoritative references used to build the protocol taxonomy:

- Linux Foundation A2A press release:
  - https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents
- Google Developers Blog A2A announcement:
  - https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/
- A2A README:
  - https://raw.githubusercontent.com/a2aproject/A2A/refs/heads/main/README.md

- BeeAI ACP OpenAPI spec:
  - https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml
- ACP docs (MCP and A2A):
  - https://agentcommunicationprotocol.dev/about/mcp-and-a2a

- Agent Client Protocol (editor ↔ coding agent) docs.rs:
  - https://docs.rs/crate/agent-client-protocol/latest

## Notes

- This file is intentionally **local-only** (untracked by default) per request; it is meant as a quick human index.
- The JSONL seed is the canonical machine-ingestable representation:
  - `docs/knowledge/kg-seeds/agentic-implementation-mindmap.2026-02-15.jsonl`

## Environment observations (2026-02-15)

These are **environment-specific** network observations captured during evidence gathering; treat them as hints and re-check in fresh environments.

- Verified via `curl` in this environment at **2026-02-15 23:53**.

- crates.io HTML pages returned 404 in this environment:
  - https://crates.io/crates/a2a-client
  - https://crates.io/crates/agent-client-protocol
- crates.io API endpoint returned 200 (JSON) for `agent-client-protocol` in this environment:
  - https://crates.io/api/v1/crates/agent-client-protocol
