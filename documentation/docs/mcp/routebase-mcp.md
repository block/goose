---
title: Routebase Extension
description: Add Routebase MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Routebase MCP Server](https://github.com/routebase-dev/routebase-mcp) as a goose extension to design, mock, test, document and monitor your APIs from one living OpenAPI spec.

Routebase keeps an API's documentation, mock servers, contract tests and monitoring derived from a single OpenAPI description, so they cannot drift apart. Through this extension goose works that same description: it creates endpoints and schemas, runs contract and security test suites, manages mock servers, publishes documentation, and reads monitoring results.

Routebase offers two ways to connect:

1. **Remote MCP server** hosted by Routebase, which signs you in to your Routebase account in the browser.
2. **Local stdio bridge** you run with `npx`, authenticated with an API key whose scopes you choose.

:::info Permissions
Every tool call runs under the same role-based permissions as your own account, so goose can never reach a project you cannot. Billing and organization access are read-only: the agent can explain a plan limit, but it cannot grant a role or spend money.
:::

## Configuration

<Tabs groupId="remote-or-local">
  <TabItem value="remote" label="Routebase Remote MCP" default>
  :::tip Quick Install
  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.routebase.dev&id=routebase&name=Routebase&description=Design%2C%20mock%2C%20test%2C%20document%20and%20monitor%20your%20APIs%20from%20one%20living%20OpenAPI%20spec)
    </TabItem>
    <TabItem value="cli" label="goose CLI">
    Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension type with:

    **Endpoint URL**
    ```
    https://mcp.routebase.dev
    ```
    </TabItem>
  </Tabs>
  :::

  :::info OAUTH FLOW
  An OAuth window will open in your browser. Sign in to Routebase and authorize access — no API key to create or paste.
  :::

  :::warning US-region accounts
  If your Routebase account lives in the US region, use `https://mcp.routebase.dev/?region=us` instead. Getting this wrong shows up as an extension with no tools rather than as an error.
  :::

  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
      <GooseDesktopInstaller
        extensionId="routebase"
        extensionName="Routebase"
        description="Design, mock, test, document and monitor your APIs from one living OpenAPI spec"
        type="http"
        url="https://mcp.routebase.dev"
      />
    </TabItem>
    <TabItem value="cli" label="goose CLI">
      <CLIExtensionInstructions
        name="routebase"
        description="Design, mock, test, document and monitor your APIs from one living OpenAPI spec"
        type="http"
        url="https://mcp.routebase.dev"
        timeout={300}
      />
    </TabItem>
  </Tabs>

  </TabItem>

  <TabItem value="local" label="Routebase Local MCP">
  :::tip Quick Install
  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
      [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=routebase-mcp&arg=--stdio&id=routebase&name=Routebase&description=Design%2C%20mock%2C%20test%2C%20document%20and%20monitor%20your%20APIs%20from%20one%20living%20OpenAPI%20spec&env=ROUTEBASE_API_KEY%3DRoutebase%20API%20Key)
    </TabItem>
    <TabItem value="cli" label="goose CLI">
      **Command**
      ```sh
      npx -y routebase-mcp --stdio
      ```

      **Environment Variable**
      ```
      ROUTEBASE_API_KEY: <YOUR_API_KEY>
      ```
    </TabItem>
  </Tabs>
  :::

  :::info Prerequisites

  - [Node.js](https://nodejs.org/) (v18.0.0 or higher) installed on your system to run this command, as it uses `npx`
  - A Routebase API key, created under **Settings → API Keys** in your Routebase workspace. Give it only the scopes you want goose to have — `specs:read` is enough to explore
  - US-region accounts also need `ROUTEBASE_REGION=us`; EU accounts need nothing
  :::

  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="routebase"
      extensionName="Routebase"
      description="Design, mock, test, document and monitor your APIs from one living OpenAPI spec"
      command="npx"
      args={["-y", "routebase-mcp", "--stdio"]}
      envVars={[
        { name: "ROUTEBASE_API_KEY", label: "Routebase API Key" }
      ]}
      apiKeyLink="https://app.routebase.dev"
      apiKeyLinkText="ROUTEBASE_API_KEY"
    />
    </TabItem>

    <TabItem value="cli" label="goose CLI">
      <CLIExtensionInstructions
        name="routebase"
        description="Design, mock, test, document and monitor your APIs from one living OpenAPI spec"
        command="npx -y routebase-mcp --stdio"
        envVars={[
          { key: "ROUTEBASE_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
        ]}
        timeout={300}
      />
    </TabItem>
  </Tabs>

  </TabItem>
</Tabs>

For all setup and configuration options, see the [Routebase MCP documentation](https://docs.routebase.dev).

## Example Usage 1

Find your way around a workspace before changing anything.

### goose Prompt

```
Which Routebase projects can I see, and what specs are in the store one?
```

### goose Output

```
I'll list your projects first, then the specs in the matching one.

routebase__list_projects returned 2 projects:

1. Acme Store — 3 environments (Development, Staging, Production)
2. Internal Tools — 2 environments

Specs in Acme Store (routebase__list_specs):

- Store API — OpenAPI 3.1, current version v2.1.0 (published)
- Checkout API — OpenAPI 3.1, current version v0.9.0 (draft)

The Store API is the published one. Want me to list its endpoints?
```

## Example Usage 2

Catch a contract that drifted away from the implementation — the thing that is hard to notice by hand.

### goose Prompt

```
Run the contract test suite for the Store API against staging and tell me what drifted.
```

### goose Output

```
Running the suite with routebase__run_test_suite, then reading the results.

Suite "Store API contract" — 24 cases, 22 passed, 2 failed.

Both failures are the same divergence, on GET /orders/{id}:

- Response body contains an undocumented field "currency" (string)
- The spec's Order schema does not declare it

The live API returns a field your consumers cannot see in the docs. Two ways
forward: add "currency" to the Order schema so the contract matches reality, or
remove it from the response if it was not meant to ship.

Adding it to the spec is a non-breaking change — I can prepare that on a spec
branch and open a merge request for review if you want.
```

## Example Usage 3

Keep the documentation portal honest once the spec changes.

### goose Prompt

```
Lint the Store API spec and publish the docs if it passes.
```

### goose Output

```
routebase__lint_spec on Store API v2.1.0: 1 warning, 0 errors.

- Warning: operation "deleteOrder" has no description (style guide rule
  "must-have-operation-description")

That is a warning, not a blocker, but it will show up as a gap in the portal.
I filled it in with a one-line description and re-ran the lint: clean.

routebase__publish_doc_version published version 2.1.0 and triggered the portal
build. It is live on your documentation domain, generated from the same spec the
tests just ran against.
```
