---
title: SignalBrain Extension
description: Add SignalBrain MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [SignalBrain MCP Server](https://github.com/whitestone1121-web/signalbrain) as a goose extension to make goose's changes verifiable: goose emits an *improvement receipt* for each change — an executable claim (what changed, the command that proves it, stated confidence) that is objectively re-scored after merge, so autonomy is earned from a measured track record.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=--from&arg=signalbrain%5Bmcp%5D&arg=sb-mcp&id=signalbrain&name=SignalBrain&description=Verifiable%20improvement%20receipts%20for%20agent%20changes)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx --from "signalbrain[mcp]" sb-mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="signalbrain"
      extensionName="SignalBrain"
      description="Verifiable improvement receipts for agent changes"
      command="uvx"
      args={["--from", "signalbrain[mcp]", "sb-mcp"]}
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="SignalBrain"
      description="Verifiable improvement receipts for agent changes"
      command='uvx --from "signalbrain[mcp]" sb-mcp'
      timeout={300}
    />
  </TabItem>

</Tabs>

## Why receipts

Agents claim "done, tests pass" faster than anyone re-checks. A receipt makes the claim executable — and the [scorer](https://github.com/whitestone1121-web/signalbrain) re-runs it after a human merges, recording `held` or failed forever. Per change-class, an agent earns auto-merge eligibility only when its last 10 high-confidence claims held. The rules were battle-tested the honest way: the reference deployment's own agents attacked the ledger, and the [public forensic record](https://github.com/whitestone1121-web/signalbrain/blob/main/docs/incidents/2026-07-tooling-trust-streak-gaming.md) reproduces from git SHAs.

The extension provides three tools:

| Tool | What it does |
|---|---|
| `emit_receipt` | Writes a spec-compliant receipt for the change goose just made — refuses guessed dates and unscoreable measure commands |
| `validate_receipt` | Checks an existing receipt against the grammar before committing |
| `gate_status` | Lets goose read its own earned-autonomy standing from the ledger |

## Example Usage

Ask goose to fix something and leave a verifiable claim behind:

### goose Prompt

> Fix the off-by-one in `pagination.py`, run the tests, and emit a receipt for the change with your honest confidence.

### Result

goose fixes the bug, then calls `emit_receipt`, producing `receipts/0001-bugfix-pagination-off-by-one.md`:

````markdown
# 0001-bugfix-pagination-off-by-one — fix page-boundary off-by-one

### How measured
```bash
python3 -m pytest tests/test_pagination.py -q
```

## Verdict

`improvement`

## Confidence
0.9
````

After the change merges, score the receipt (note: the goose extension runs via `uvx`, which does not put the `sb` CLI on your PATH — invoke it the same way):

```sh
uvx --from signalbrain sb score receipts/0001-bugfix-pagination-off-by-one.md \
  --root . --ledger .signalbrain/ledger.jsonl
```

The scorer re-runs that pytest command. If it passes, the claim *held*; if not, the miss is recorded against goose's calibration — permanently. Ten held high-confidence claims in a class, and goose has earned auto-merge eligibility there.

The receipt format is an open spec ([RECEIPT_SPEC](https://github.com/whitestone1121-web/signalbrain/blob/main/docs/RECEIPT_SPEC.md), Apache-2.0), and the CI side is a [GitHub Action](https://github.com/whitestone1121-web/signalbrain/blob/main/action.yml) — your repo, your ledger, no server.
