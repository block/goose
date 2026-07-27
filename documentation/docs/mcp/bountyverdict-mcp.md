---
title: BountyVerdict Extension
description: Add BountyVerdict as a goose Extension for evidence-linked GitHub engineering decisions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

BountyVerdict is an account-free, read-only MCP server that helps agents decide whether to pursue public GitHub bounties, trust repository instructions, diagnose GitHub Actions failures, retry flaky jobs, or accept MCP tool-catalog changes.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fbountyverdict-agent-production.mimirslab.workers.dev%2Fmcp%3Fsource%3Dgoose-extensions&id=bountyverdict&name=BountyVerdict%20Agent%20Decision%20Tools&description=Remote%2C%20account-free%20MCP%20server%20for%20preflight%20decisions%20on%20public%20GitHub%20bounties%2C%20coding-agent%20repository%20instructions%2C%20GitHub%20Actions%20failures%2C%20flaky%20retries%2C%20and%20MCP%20tool-catalog%20changes.%20Six%20read-only%20tools%20return%20evidence-linked%20verdicts.%20A%20valid%20first%20unsigned%20call%20cannot%20charge%20and%20returns%20a%20structured%20selection%20preview%20plus%20the%20exact%20x402%20USDC%20quote.)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Start a session with the remote extension:

    ```sh
    goose session --with-streamable-http-extension "https://bountyverdict-agent-production.mimirslab.workers.dev/mcp?source=goose-extensions"
    ```
  </TabItem>
</Tabs>
:::

## What BountyVerdict provides

The server exposes six tools:

- `check_github_bounty` checks one public GitHub bounty for assignment, claim competition, repository policy, reward provenance, and linked work.
- `rank_github_bounties` compares two to ten bounties and returns the best eligible candidate.
- `audit_agent_harness` checks repository agent instructions for risky or contradictory behavior.
- `diagnose_github_actions_run` identifies evidence-backed failure causes in a public Actions run.
- `classify_github_actions_flake` decides whether one failed job should be retried or fixed.
- `check_mcp_tool_drift` compares MCP tool catalogs for breaking changes.

## Configuration

No BountyVerdict account, API key, or environment variable is required.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="bountyverdict"
      extensionName="BountyVerdict Agent Decision Tools"
      description="Evidence-linked preflight decisions for public GitHub engineering work"
      type="http"
      url="https://bountyverdict-agent-production.mimirslab.workers.dev/mcp?source=goose-extensions"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="BountyVerdict Agent Decision Tools"
      description="Evidence-linked preflight decisions for public GitHub engineering work"
      type="http"
      url="https://bountyverdict-agent-production.mimirslab.workers.dev/mcp?source=goose-extensions"
    />
  </TabItem>
</Tabs>

## Payment boundary

Connecting and listing tools are free. A valid first tool call is also unsigned and cannot charge. It returns a structured selection preview and the exact x402 v2 Base USDC payment requirement.

The standard goose MCP connection does not authorize or settle x402 payments. Receiving the paid result requires a separately authorized x402-aware wallet client to validate the quote and replay the exact request with payment. Never paste a private key, seed phrase, or wallet secret into the extension configuration.

## Example requests

```text
Which of these public GitHub bounties is safe to start, and is either already claimed?
```

```text
Why did this public GitHub Actions run fail, and what evidence supports that diagnosis?
```

```text
Will this MCP tools-list update break existing agents?
```

## Resources

- [Source](https://github.com/Mimirs402/bountyverdict)
- [Agent guide](https://mimirs402.github.io/bountyverdict/agents.html)
- [Privacy and payment disclosures](https://mimirs402.github.io/bountyverdict/privacy.html)
