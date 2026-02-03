<p align="center">
  <img src="frontend/public/aci-dev-full-logo-light-bg.svg" alt="ACI.dev Logo" width="100%">
</p>

# Gate22: Open-Source MCP Gateway and Control Plane

<p align="center">
  <a href="https://www.aci.dev/products/gate22"><img src="https://img.shields.io/badge/Documentation-34a1bf" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/Apache-2.0"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License"></a>
  <a href="https://discord.com/invite/UU2XAnfHJh"><img src="https://img.shields.io/discord/1349424813550342275?logo=discord&label=Discord&color=7289DA" alt="Discord"></a>
  <a href="https://x.com/AipoLabs"><img src="https://img.shields.io/twitter/follow/AipoLabs?style=social" alt="Twitter Follow"></a>
</p>

> [!NOTE]
> This repo is for **Gate22**. If you're looking for the **Tool-calling Platform**, see [ACI](https://github.com/aipotheosis-labs/aci).

Govern which tools agents can use, what they can do, and how it’s audited—across agentic IDEs like Cursor, or other agents and AI tools.

**Gate22 is built for engineering organizations and teams** (Platform/Infra/DevEx, Security, Data/Analytics Eng). Admins onboard any remote MCP server (internal or external), set credential modes (org-shared or per-user), and define function-level allow lists per configuration. Developers then compose their own bundles from MCP configurations they’re permitted to use and expose them through a single unified MCP endpoint with just two functions — search and execute. Even if a bundle spans 20 MCPs/400+ tools, the unified endpoint discovers tools at call time, keeping IDE context lean while enforcing permissions.

![Gate22 Architecture](frontend/public/gate22-architecture-light-bg.svg)

<p align="center">
  Join us on <a href="https://discord.com/invite/UU2XAnfHJh">Discord</a> to help shape the future of AI governance.<br/><br/>
  🌟 <strong>Star Gate22 to stay updated on new releases!</strong><br/><br/>
  <a href="https://github.com/aipotheosis-labs/mcp-gateway/stargazers">
    <img src="https://img.shields.io/github/stars/aipotheosis-labs/mcp-gateway?style=social" alt="GitHub Stars">
  </a>
</p>

## 💻 Getting Started: Local Development

To run the full Gate22 platform (backend server and frontend portal) locally, follow the individual README files for each component:

- **Backend:** [backend/README.md](backend/README.md)
- **Frontend:** [frontend/README.md](frontend/README.md)

## 👥 Who Should Use This Now

- **Platform / Infra / DevEx teams** rolling out agentic IDEs or internal AI agents at org scale.
- **Security / GRC** that need least-privilege execution and auditability for agent tool-use.
- **Data & Analytics Engineering** that want governed access to internal tools/BI with minimal setup.

**Not a great fit (yet):** single-user hobby setups; research sandboxes that don’t need governance.

## 📺 Demo Video

[![ACI.dev **Gate22** Tutorial](frontend/public/gate22-banner.png)](https://youtu.be/iYNtp5TSAqw)

## ✨ Key Features in v0

- Function allow-list permissioning (per MCP configuration).
- Admin-set credential modes: org-shared or per-user (admins may publish both variants through separate MCP configurations of the same MCP server).
- User-created bundles (private for now) → one endpoint (remote MCP URL) per bundle and only accessible to the bundle creator.
- MCP tool list refresh & diff view (see what changed for an MCP server before you use it).
- MCP bundles condense any number of MCPs and tools into just a single MCP endpoint and two functions--search and execute--to save context window.

## 💡 Why Use Gate22?

Maximize your ROI from AI tools through safe integrations with any MCP and have visibility and audit.

- **Simple, least-privileged setup**: admins set MCP permissions; developers only see/execute what they’re allowed.
- **No context bloat**: two function surface (search/execute) dynamically resolves tools at runtime.
- **Separation of duties**: admins control configs/credentials; developers assemble bundles from permitted building blocks.
- **Extend AI tools**: have ease of mind to let AI tools interact with the rest of your stack.

## 🧰 Example

An admin connects Notion, Supabase, and Render MCPs as read-only for everyone, and adds internal MCPs with write actions limited to specific projects. Developers create their own bundles (from what they’re entitled to), link one endpoint in their IDE, and safely execute only allowed functions—every call audited.

## 🔗 Quick Links

- **Cloud Version:** [gate22.aci.dev](https://gate22.aci.dev/)
- **Documentation:** [aci.dev/docs](https://gate22-docs.aci.dev/)
- **Blog:** [aci.dev/blog](https://www.aci.dev/blog)
- **Community:** [Discord](https://discord.com/invite/UU2XAnfHJh) | [Twitter/X](https://x.com/AipoLabs) | [LinkedIn](https://www.linkedin.com/company/aci-dev-by-aipolabs/posts/?feedView=all)

## 🗺️ Near-Term Roadmap

- Tool-call logs: per-call records (who/what/when/result/latency) with export.
- Policy enforcement (P0): thin, policy-as-code starter (env/time-box/allow-list), fail-closed for risky ops.
- MCP tool-change audit: persist diffs when servers refresh; searchable history.
- Bundle sharing: opt-in sharing within team/workspace (role-scoped visibility).
- Security hardening: pre-flight checks for tool poisoning / prompt-injection patterns on MCP servers/configs.

## 🔭 Future (design RFCs)

- Policy-as-code v2 (OPA/Cedar-style ABAC, approvals integration).
- Quotas & budgets (per user/team/app/function).
- Compliance (SIEM export, immutable audit logs).
- Templates (“golden bundles”) for common stacks (Data/BI, SDLC, On-call).

## 👋 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for more information.
