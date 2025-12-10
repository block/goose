# MCP Apps Implementation Notes for Goose

**Date Started:** 2025-12-05  
**Specification:** [SEP-1865: MCP Apps](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx)  
**Objective:** Implement MCP Apps support in Goose (Desktop + CLI)

---

## Specification Summary

- **Extension ID:** `io.modelcontextprotocol/ui`
- **URI Scheme:** `ui://`
- **Content Type (MVP):** `text/html;profile=mcp-app`
- **Communication:** JSON-RPC 2.0 over `postMessage` (iframe ↔ host)

---

## Reading Notes & Ideas

### Key Insight: iframes are MCP Clients

The iframe isn't just a dumb renderer - it's actually an MCP client that can:
- Call `tools/call` to execute tools on the server
- Call `resources/read` to fetch resources
- Send `ui/message` to inject messages into the conversation
- Do the whole `ui/initialize` → `ui/notifications/initialized` handshake

**Architecture:**
```
MCP Server <---> Goose (Host) <---> iframe (MCP Client)
                     ^
                     |
              postMessage bridge
```

Goose becomes a **broker** between the iframe and the MCP server:
1. Listen for `postMessage` from the iframe
2. Validate the JSON-RPC messages
3. Proxy allowed requests (`tools/call`, `resources/read`, etc.) to the MCP server
4. Send responses back to the iframe

This is where the security model really matters - Goose is the gatekeeper deciding what the iframe is allowed to do.

---

### Deprecation Strategy (MCP-UI → MCP Apps)

- **Detection approach:** Currently we check if URI starts with `ui://` prefix. We can also look for the new MIME type `text/html;profile=mcp-app` to distinguish between old MCP-UI and new MCP Apps.
- **Code path fork:** Use MIME type detection to load a totally different renderer for MCP Apps, leaving existing MCP-UI code in place during transition.
- **Warning banner update:** Update the existing "experimental" warning bar for MCP-UI to include:
  - Clearer messaging that MCP-UI is being deprecated
  - A target date for when MCP-UI support will be removed
  - Encourage extension authors to migrate to the new MCP Apps spec



---

## Open Questions

- Does the Rust MCP SDK handle `resources/read` requests? (Need to verify before implementing UI resource fetching)

- Does the Rust MCP SDK's resource response include all the fields we need?
  - `uri` ✅
  - `mimeType` ✅ (as `mime_type: Option<String>`)
  - `text` or `blob` ✅ (enum variants `TextResourceContents` / `BlobResourceContents`)
  - `_meta` ✅ (as `meta: Option<Meta>` where Meta wraps JsonObject)
    - `ui.csp.connectDomains` - Access via `meta.0.get("ui")...`
    - `ui.csp.resourceDomains` - Access via `meta.0.get("ui")...`
    - `ui.domain` - Access via `meta.0.get("ui")...`
    - `ui.prefersBorder` - Access via `meta.0.get("ui")...`
  - **CONFIRMED:** All fields available in rmcp 0.9.1

- Does Goose need to validate that the `text` or `blob` content is a valid HTML5 document? Or just trust the MIME type and render it?

- Does Goose need to prefetch and cache UI resources for performance optimization? (Spec mentions hosts MAY do this)
  - **Follow-up idea:** Prefetching could enable security review at connection time - scan HTML for suspicious patterns and surface warnings in the Goose UI before the tool is ever invoked. Pursue this as a fast-follow after MVP is working.

- **RPC methods:** There are a lot of JSON-RPC methods to implement (lifecycle, notifications, requests). Will there be a client library that handles this, or do we need to build it ourselves? Look into:
  - `ui/initialize`, `ui/notifications/initialized`
  - `ui/notifications/tool-input`, `ui/notifications/tool-input-partial`, `ui/notifications/tool-result`
  - `ui/tool-cancelled`
  - `ui/resize`, `ui/message`
  - `tools/call`, `resources/read`, `ping`
  - Check if MCP-UI SDK or a new official SDK will provide helpers for this

- **Hiding redundant text when UI is shown:** The spec separates `content` (text for model context / text-only hosts) from `structuredContent` (for UI rendering). But it doesn't explicitly say hosts SHOULD hide the text `content` when rendering a UI. Should Goose suppress the text output when displaying an MCP App to avoid redundancy? Worth raising for clarification in the spec.

- **Private tools:** The spec mentions something about private tools - worth noting as a concept to be aware of.



---

## Implementation Considerations

### Desktop (Electron)

- **Sandbox proxy:** The spec has a pretty exhaustive checklist for sandbox proxy requirements (different origins, permissions, message forwarding, CSP enforcement, etc.). Should be relatively straightforward to ensure we're complying with everything - just follow the checklist.
  - **Must implement:**
    - Sandbox sends `ui/notifications/sandbox-ready` when ready
    - Host sends raw HTML via `ui/notifications/sandbox-resource-ready` once sandbox is ready

- **HostContext interface:** We've started sending `theme` and `platform` to the UI. Need to expand this to the full `HostContext` interface from the spec:
  - `toolInfo`, `theme`, `displayMode`, `availableDisplayModes`, `viewport`, `locale`, `timeZone`, `userAgent`, `platform`, `deviceCapabilities`, `safeAreaInsets`
  - **Confusion about `platform`:** The spec comment says it's for "responsive design" but the options (`web`, `desktop`, `mobile`) seem more about the host environment (browser vs Electron vs mobile app), not screen size. Need clarity.
    - **Action:** Check existing issues/discussions on the ext-apps repo to see if this has been raised. If not, open an issue to clarify the intent of `platform`.

- **hostInfo:** This is where we identify ourselves. Set `name: "Goose Desktop"` and include the version number.

- **`prefersBorder` handling:** When a UI resource has `prefersBorder: true`, we can respond by:
  - Removing all padding from the current container
  - Possibly removing the background element entirely
  - Let the iframe content define its own visual boundaries

- **CSP enforcement:** Map the CSP settings from the server's resource `_meta.ui.csp` to the HTML proxy's `<head>` (via `<meta http-equiv="Content-Security-Policy">`). Need to construct the CSP string from `connectDomains` and `resourceDomains` arrays.
  - Default to restrictive (block everything not explicitly declared)
  - MUST NOT allow undeclared domains
  - **Question:** Spec mentions "audit trail" - should log CSP configs for review. Does this just mean writing to goose logs? Or something more formal?

- **Future idea: "Stats for Nerds" for MCP Apps** - A playful way to let users inspect/hover over an MCP App to see resource details like:
  - Resource URI
  - Declared `connectDomains` and `resourceDomains`
  - Extension name / source
  - Could be a little info icon or expandable panel - make it fun and on-brand for Goose



### CLI
<!-- Notes specific to CLI - likely limited/no UI support? -->



### Core (goose crate)

- **Capability negotiation:** Goose must advertise MCP Apps support in its `initialize` request to MCP servers:
  ```json
  {
    "capabilities": {
      "extensions": {
        "io.modelcontextprotocol/ui": {
          "mimeTypes": ["text/html;profile=mcp-app"]
        }
      }
    }
  }
  ```
  - This tells servers that Goose can render UI resources
  - Servers that don't see this capability should fall back to text-only responses

- **Tool → Resource linking:** When a tool is initialized and has `_meta["ui/resourceUri"]`, we need to:
  1. Detect that metadata field on the tool
  2. Use the Rust MCP SDK to call `resources/read` with that URI
  3. Fetch the UI resource content (HTML + metadata) to pass to the renderer
  - This needs to happen at tool initialization time, not tool execution time (for prefetching/caching)



---

## goose-server
<!-- API/route changes needed -->



---

## Server-Side Notes (Axon / Agentic Commerce)

- **Capability-based response routing:** Our server will know if a host (like Goose) supports MCP Apps via the `extensions["io.modelcontextprotocol/ui"]` capability in the initialize request.
- **Dual-mode support:** This gives us a way to maintain two strategies side-by-side:
  1. **OpenAI ChatGPT SDK + MCP-UI** - Current approach for hosts that don't support the new spec
  2. **MCP Apps (SEP-1865)** - New approach for hosts that advertise `text/html;profile=mcp-app` support
- **What changes per mode:**
  - Resource registration strategy
  - Tool result embedding (how we return `content` vs `structuredContent`)
  - Possibly different UI resource URIs or templates
- **Implementation:** Set up conditional logic at initialization time to route to the appropriate resource registration strategy based on detected host capabilities. Don't know exactly how to do this yet, but that's the goal.

- **Simpler alternative:** Maybe we can just drop ChatGPT SDK support and drop MCP-UI entirely. Follow the spec's recommendation:
  - Check if host has `io.modelcontextprotocol/ui` capability
  - If yes → register the UI-enabled tools (with `ui/resourceUri` metadata) and resources
  - If no → don't register those tools/resources at all, fall back to text-only tools
  - This is cleaner than maintaining dual-mode logic. Just one path forward.

---

## Dependencies & Related Work

- Goose is already listed as an MCP-UI adopter in the spec
- Reference: MCP-UI project (mcpui.dev)
- **Prerequisite: SEP-1724** - Goose must implement client-server capability negotiation (extensions capability mechanism) before MCP Apps can work. Need to verify if this is already shipped or needs to be added.
  - **Finding:** The `rmcp` crate (even v0.10.0) does NOT have an `extensions` field on `ClientCapabilities`. It only has: `experimental`, `roots`, `sampling`, `elicitation`.
  - **Options:**
    1. Use `experimental` field as a workaround (it's a `BTreeMap<String, JsonObject>`)
    2. Contribute the `extensions` field to rmcp upstream
    3. Fork/patch rmcp locally
  - Need to decide which approach to take

---

## Action Items

- [ ] Review existing MCP implementation in Goose
- [ ] Identify where UI resource handling would fit
- [ ] Design iframe sandbox approach for Electron
- [ ] Plan capability negotiation implementation
- [ ] ...

