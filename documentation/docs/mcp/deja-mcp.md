---
title: deja Extension
description: Add deja as a goose Extension to recall past sessions from every coding agent on your machine
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add [deja](https://github.com/vshulcz/deja-vu) as a goose extension. deja indexes the session transcripts that coding agents on your machine already write to disk — Claude Code, Codex, Cursor, opencode, Zed and others — and serves them over MCP, so goose can answer what you did about something before, including work from before deja was installed.

The index is local BM25 over those files. No LLM, no embeddings, and nothing leaves the machine.

## Configuration

:::info
deja is a single Go binary rather than an npm or PyPI package, so it is installed first and the extension then runs it. There is no one-click installer for this one.
:::

Install the binary:

```sh
brew install vshulcz/tap/deja-vu
```

or

```sh
curl -fsSL https://raw.githubusercontent.com/vshulcz/deja-vu/main/install.sh | sh
```

Then let deja write the extension into goose's `config.yaml` and build the first index:

```sh
deja install goose
```

That is the whole setup. To add the extension by hand instead:

<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI" default>
    <CLIExtensionInstructions
      name="deja"
      description="Search the sessions your other coding agents already wrote to disk"
      type="stdio"
      command="deja mcp"
      timeout={300}
    />
  </TabItem>
  <TabItem value="ui" label="goose Desktop">
    Add a `deja` entry under `extensions` in `~/.config/goose/config.yaml`:

    ```yaml
    extensions:
      deja:
        enabled: true
        type: stdio
        name: deja
        cmd: deja
        args:
          - mcp
        timeout: 300
    ```

    `deja install goose` writes exactly this block, keeping the rest of the file — including comments — as it was.
  </TabItem>
</Tabs>

## Example Usage

deja is most useful for the question a fresh session cannot answer: what happened last time. The history it reads was written by whatever agent you were using at the time, so the answer can come from a tool you are no longer running.

### goose Prompt

```
I'm getting "connection pool exhausted" from the worker again. Have I hit this before?
```

### goose Output

```
─── recall | deja ──────────────────────────
query: connection pool exhausted

Found 2 earlier sessions.

[claude] api · Jul 8 — the same error under load tests. The pool was sized from
MaxIdleConns while the worker opened one connection per job; fixed by moving the
worker onto the shared pool and capping concurrency at pool size.

[codex] api · Jun 30 — same message, different cause: a migration held a
transaction open. Not this one, going by the stack you pasted.

You solved this in July. The fix was to stop the worker from opening its own
connections — it now takes them from the shared pool, with concurrency capped at
the pool size. Do you want me to check whether that worker still does that?
```

The other tools in the extension answer the neighbouring questions: `blame` for the sessions that discussed a file before you edit it, `fix` for what this machine ran after a given error in the sessions where it did not come back, `how` for the real invocation of a build or test with the flags actually used, and `remember` for storing one decision worth keeping.

Automatic recall is available too — `deja install goose-auto` refreshes `.goosehints` at session start, so the project's recent history is in front of the model without anyone asking for it.
