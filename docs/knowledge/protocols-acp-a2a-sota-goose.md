# SOTA: ACP vs A2A vs agent-client-protocol (meanings, implications, Goose + KG)

Last updated: 2026-02-16

## Meanings (disambiguation)

The acronym **ACP** is overloaded. In Goose docs/code and in our KG, use:

- **Agent Communication Protocol (ACP)**: BeeAI / i-am-bee protocol (REST + OpenAPI).
- **Agent2Agent Protocol (A2A)**: Linux Foundation A2A project (JSON-RPC 2.0 over HTTP(S)).
- **agent-client-protocol**: Rust crate / client (e.g., editor) ↔ agent protocol; distinct from Agent Communication Protocol.

## Evidence (primary sources)

### ACP is now part of A2A

> ore exploring further. # Welcome > Get to know the Agent Communication Protocol <Info> ## 🚀 IMPORTANT UPDATE **ACP is now part of A2A under the Linux Foundation!** 👉 [Learn more](https://github.com/orgs/i-am-bee/discussions/5) | 🛠️ [Migration Guide

Source: https://agentcommunicationprotocol.dev/introduction/welcome

### ACP is a REST/OpenAPI-described protocol

> openapi: 3.1.1 info: title: ACP - Agent Communication Protocol description: >- The Agent Communication Protocol (ACP) provides a standardized RESTful API for managing, orchestrating, and executing AI agents. It suppo

Source: https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml

> openapi: 3.1.1 info: title: ACP - Agent Communication Protocol description: >- The Agent Communication Protocol (ACP) provides a standardized RESTful API for managing, orchestrating, and executing AI agents. It supports synchronous, asynchronous, and streamed agent interactions, with both stateless and stateful execution modes. license: name: Apache 2.0 url: ht

Source: https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml

### ACP discovery uses an Agent Manifest

> //agentcommunicationprotocol.dev/llms.txt > Use this file to discover all available pages before exploring further. # Agent Manifest > Structure and usage of the Agent Manifest The **Agent Manifest** describes essential properties of an agent, including its identity, capabilities, metadata, and runtime status. It also plays an important role in discoverability and how the ACP server adv

Source: https://agentcommunicationprotocol.dev/core-concepts/agent-manifest

### A2A is a Linux Foundation project (created by Google)

> rce Summit North America – June 23, 2025</strong> – <a href="https://hubs.la/Q03pfzmC0"><span>The Linux Foundation</span></a>, the nonprofit organization enabling mass innovation through open source, today announced the launch of the Agent2Agent (A2A) project, an open protocol created by Google for secure agent-to-agent communication and collaboration. Developed to address the challenges of scaling AI agents across enterprise environments, A2A empowers developers to build ag

Source: https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents

### A2A discovery uses Agent Cards

> take the correct action. This interaction involves several key capabilities:</p><p data-block-key="eb0pg"></p><ul><li data-block-key="2h8en"><b>Capability discovery:</b> Agents can advertise their capabilities using an “Agent Card” in JSON format, allowing the client agent to identify the best agent that can perform a task and leverage A2A to communicate with the remote agent.</li></ul><p data-block-key="bv10u"></p><ul><li data-block-key="7sco4">

Source: https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/

### A2A uses JSON-RPC 2.0 over HTTP(S)

> ity:** Allow agents to collaborate without needing to share internal memory, proprietary logic, or specific tool implementations, enhancing security and protecting intellectual property. ### Key Features - **Standardized Communication:** JSON-RPC 2.0 over HTTP(S). - **Agent Discovery:** Via "Agent Cards" detailing capabilities and connection info. - **Flexible Interaction:** Supports synchronous request/response, streaming (SSE), and asynchronous push notifications. - **Rich Data Excha

Source: https://raw.githubusercontent.com/a2aproject/A2A/main/README.md

## Canonical A2A spec/schema (code artifacts)

In this environment, the JS-rendered a2aprotocol.org docs/resources pages frequently fail to load fully. The most reliable **canonical, machine-readable** A2A schema artifacts we can fetch are in the A2A GitHub repo as **Protocol Buffers** under `specification/`.

- Protobuf schema: https://raw.githubusercontent.com/a2aproject/A2A/main/specification/a2a.proto
- Buf module config: https://raw.githubusercontent.com/a2aproject/A2A/main/specification/buf.yaml

Evidence snippets:

> // Older protoc compilers don't understand edition yet. syntax = "proto3"; package a2a.v1; import "google/api/annotations.proto"; import "google/api/client.proto"; import "google/api/

Source: https://raw.githubusercontent.com/a2aproject/A2A/main/specification/a2a.proto

> // Older protoc compilers don't understand edition yet. syntax = "proto3"; package a2a.v1; import "google/api/annotations.proto"; import "google/api/client.proto"; import "google/api/field_behavior.pro

Source: https://raw.githubusercontent.com/a2aproject/A2A/main/specification/a2a.proto

> version: v2 deps: # Common Protobuf types. - buf.build/googleapis/googleapis lint: use: # Indicates that all the defau

Source: https://raw.githubusercontent.com/a2aproject/A2A/main/specification/buf.yaml

## Double-check: “A2A from IBM” vs “A2A from Linux Foundation”

What we can assert from primary sources:

- ACP’s own docs state ACP is now part of **A2A under the Linux Foundation** (see ACP welcome page above).
- The Linux Foundation announcement positions **A2A** as an LF-hosted project (and describes it as created by Google).

Conclusion: model A2A as **one** protocol/project under the Linux Foundation. If IBM is referenced elsewhere, treat it as a participant/partner/contributor rather than a distinct competing A2A.

## Functional overlap (ACP ↔ A2A)

Overlaps:

- Discovery/self-description: **ACP Agent Manifest** ↔ **A2A Agent Card**.
- Long-running work tracking: **ACP runs** ↔ **A2A tasks**.
- Both ecosystems describe sync + async + streaming patterns.

Key differences:

- Transport/shape: **ACP is REST + OpenAPI**; **A2A is JSON-RPC 2.0 over HTTP(S)**.
- Canonical schemas: ACP publishes OpenAPI; A2A appears to publish protobuf schemas (plus narrative docs).

## Implications for Goose

Documentation hygiene:

- Never write “ACP” unqualified in Goose docs. Use “Agent Communication Protocol (ACP)” or “agent-client-protocol” explicitly.
- Prefer “A2A” when discussing agent-to-agent interoperability unless you are explicitly referencing BeeAI/i-am-bee ACP artifacts.

KG usage:

- Use the KG node **`ACP (acronym collision)`** as the entry point for ambiguous “ACP” queries.
- Canonical schema nodes added: `A2A schema (specification/a2a.proto)`, `A2A schema (specification/buf.yaml)`, `A2A schema (specification/buf.gen.yaml)`.

## How-to: orchestration patterns (framework-agnostic)

### Orchestrating ACP agents (REST/OpenAPI)

1. Discover agents and retrieve each agent’s manifest.
2. Select an agent using manifest metadata (capabilities/content types/etc.).
3. Start a run (sync/async/streaming depending on server support).
4. Monitor run state (poll or stream updates).
5. Normalize outputs into Goose’s internal message/artifact abstractions for CLI/UI.

Reference: https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml

### Orchestrating A2A agents (JSON-RPC 2.0)

1. Obtain Agent Cards (registry or direct discovery).
2. Choose a remote agent based on card capabilities.
3. Create/manage tasks via JSON-RPC calls.
4. Handle streaming updates when supported.
5. Persist artifacts (outputs) and present them via Goose CLI/UI.

References:

- https://raw.githubusercontent.com/a2aproject/A2A/main/README.md
- https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/
