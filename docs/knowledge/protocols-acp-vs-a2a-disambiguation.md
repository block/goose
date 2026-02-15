# Protocol disambiguation: ACP (Agent Communication Protocol) vs ACP (Agent Client Protocol) vs A2A

This note captures **verified** primary-source facts (with citations) about multiple, unrelated protocols that share the acronym **“ACP”**, plus the **A2A (Agent2Agent)** protocol. The goal is to make future research/audits/debugging faster by preventing acronym confusion.

## Mind map (retrieval-first)

```mermaid
mindmap
  root((Inter-agent protocols / "ACP" acronym collision))
    A2A["A2A = Agent2Agent Protocol"]
      What["Goal: agent-to-agent interoperability"]
      Origins["Created by Google; now hosted under Linux Foundation"]
      Evidence
        LFPress["LF press release: open protocol created by Google" ]
        A2AReadme["A2A README: open protocol; LF project; contributed by Google" ]
        GoogleBlog["Google Dev Blog: launching A2A; complements MCP" ]
      KeyConcepts
        AgentCard["Agent Card"]
        Task["Task lifecycle"]
        Artifact["Artifact output"]
        Transport["JSON-RPC 2.0 over HTTP(S) (per README)" ]

    ACP_Comms["ACP = Agent Communication Protocol (agent-to-agent)"]
      Who["IBM / BeeAI ecosystem"]
      Evidence
        ACPDocs["agentcommunicationprotocol.dev pages"]
        OpenAPISpec["OpenAPI title: ACP - Agent Communication Protocol"]
        LFIBMPress["LF/IBM: BeeAI powered by open ACP"]
      Definition["Enables communication between agents"]
      API
        Agents["/agents endpoints"]
        Runs["/runs endpoints"]
        Sessions["/sessions endpoints"]
        Events["/runs/{run_id}/events"]

    ACP_Client["ACP = Agent Client Protocol (editor <-> coding agent)"]
      Who["agentclientprotocol.com ecosystem"]
      Definition["Standardizes communication between code editors and AI coding agents"]
      Evidence
        DocsRS["docs.rs crate description"]
      Distinguish["Not the same as Agent Communication Protocol"]

    Guidance["Naming / disambiguation rule"]
      UseA2A["Use 'A2A' for agent-to-agent (Google/LF)."]
      UseACPComms["Use 'ACP (Agent Communication Protocol)' or 'BeeAI ACP' for i-am-bee/acp."]
      UseACPClient["Use 'Agent Client Protocol (ACP)' only in editor/coding-agent contexts."]
```

## Verified facts (with primary sources)

### A2A = Agent2Agent Protocol

- Linux Foundation press release: A2A is an **open protocol created by Google** for secure agent-to-agent communication/collaboration, and the A2A project launch is dated **June 23, 2025** on the press-release page. The page links to the A2A GitHub repo: https://github.com/a2aproject/A2A
  - Source: https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents
  - Local HTML (repro): `/tmp/lf_a2a.html`

- A2A README:
  - “**An open protocol enabling communication and interoperability between opaque agentic applications.**”
  - The README describes A2A as “an open source project under the Linux Foundation, contributed by Google” and notes it is “distributed under the **Apache 2.0 License**.”
  - Technical notes from the README:
    - Transport: JSON-RPC 2.0 over HTTP(S)
    - Discovery: agents advertise capabilities via “Agent Cards”
    - Execution model: task lifecycle; task outputs are “artifacts”
  - Source: https://raw.githubusercontent.com/a2aproject/A2A/refs/heads/main/README.md
  - Local copy (repro): `/tmp/a2a_readme.md`

- Google Developers Blog: “launching a new, open protocol called Agent2Agent (A2A)” and notes A2A complements Anthropic’s Model Context Protocol (MCP). The post also describes:
  - A JSON “Agent Card” for capability advertisement
  - A task lifecycle including long-running tasks
  - Task outputs called “artifacts”
  - A model where client agents invoke remote agents
  - Source: https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/

### ACP = Agent Communication Protocol (agent-to-agent; BeeAI / IBM ecosystem)

- ACP OpenAPI spec:
  - Declares `openapi: 3.1.1`
  - `info.title: ACP - Agent Communication Protocol` (and another `title` occurrence later in the spec)
  - Source: https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml
  - Local copy (repro): `/tmp/beeai_acp_openapi.yaml`

- ACP docs page (`MCP and A2A`) states: “Agent Communication Protocol (ACP) is a protocol that enables **communication between agents**.”
  - Source: https://agentcommunicationprotocol.dev/about/mcp-and-a2a
  - Local HTML (repro): `/tmp/acp_mcp_and_a2a.html`

- The same ACP docs page distinguishes ACP vs A2A by noting ACP (IBM, March 2025) and A2A (Google, April 2025) both aim to standardize agent-to-agent communication.
  - Source: https://agentcommunicationprotocol.dev/about/mcp-and-a2a
  - Local HTML (repro): `/tmp/acp_mcp_and_a2a.html`

- Linux Foundation / IBM press page (BeeAI) includes a line stating BeeAI is “Powered by the open Agent Communication Protocol (ACP)”.
  - Source: https://www.linuxfoundation.org/press/ai-workflows-get-new-open-source-tools-to-advance-document-intelligence-data-quality-and-decentralized-ai-with-ibms-contribution-of-3-projects-to-linux-fou-1745937200621

### ACP = Agent Client Protocol (editor <-> coding agent)

- docs.rs crate description for `agent-client-protocol` indicates it is “A protocol for standardizing communication between **code editors** and **AI coding agents**,” and uses the acronym ACP for “Agent Client Protocol”.
  - Source: https://docs.rs/crate/agent-client-protocol/latest

- The crate’s homepage/source repository is the `agentclientprotocol/rust-sdk` GitHub repo.
  - Source: https://github.com/agentclientprotocol/rust-sdk

- The docs.rs page also links to Agent Client Protocol SDK repos in other languages:
  - Kotlin SDK: https://github.com/agentclientprotocol/kotlin-sdk
  - Python SDK: https://github.com/agentclientprotocol/python-sdk
  - TypeScript SDK: https://github.com/agentclientprotocol/typescript-sdk
  - Source: https://docs.rs/crate/agent-client-protocol/latest

## Terminology rules (recommended)

1. Prefer **A2A** when you mean *agent-to-agent interoperability protocol* (Google-originated; Linux Foundation-hosted).
2. Say **ACP (Agent Communication Protocol)** or **BeeAI ACP** when referencing `i-am-bee/acp` and the `agentcommunicationprotocol.dev` docs.
3. Say **Agent Client Protocol (ACP)** only in editor/coding-agent contexts (e.g., docs.rs `agent-client-protocol`).

## Local evidence artifacts (repro pointers)

These files were fetched to avoid `curl | head` broken-pipe truncation and to allow later re-grepping/parsing:

- ACP Mintlify pages (examples):
  - `/tmp/acp_site_welcome.html` (from https://agentcommunicationprotocol.dev/introduction/welcome)
  - `/tmp/acp_mcp_and_a2a.html` (from https://agentcommunicationprotocol.dev/about/mcp-and-a2a)

- Saved large command outputs:
  - `/tmp/goose_mcp_responses/mcp_response_20260215_192112.423288.txt`
  - `/tmp/goose_mcp_responses/mcp_response_20260215_192112.423405.txt`
  - `/tmp/goose_mcp_responses/mcp_response_20260215_192134.820467.txt`
  - `/tmp/goose_mcp_responses/mcp_response_20260215_192134.820613.txt`

## Registry status checks (observations)

- As of **2026-02-15**, HTTP requests in this environment to these crates.io pages returned **404 Not Found**:
  - https://crates.io/crates/a2a-client
  - https://crates.io/crates/agent-client-protocol

## Open questions / non-claims

- We do **not** currently have a primary source that explicitly states “Agent Communication Protocol (ACP) renamed/migrated to A2A.” The ACP docs describe them as distinct efforts launched by different orgs.
  - Source: https://agentcommunicationprotocol.dev/about/mcp-and-a2a
