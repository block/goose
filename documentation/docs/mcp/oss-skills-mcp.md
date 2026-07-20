---
title: OSS Skills Extension
description: Add the OSS Skills MCP server as a goose extension for guided open-source contribution workflows
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [OSS Skills](https://github.com/chiruu12/oss-skills-mcp) server as a goose extension. It bundles 15 guided workflows for contributing to open source — from finding your first issue to becoming a regular contributor.

The skills are designed to keep you in the loop: goose does the research (reading contribution docs, exploring codebases, analyzing issues) and presents the context, while you make the decisions and write the code.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=--from&arg=git%2Bhttps%3A%2F%2Fgithub.com%2Fchiruu12%2Foss-skills-mcp&arg=oss-skills-mcp&id=oss-skills-mcp&name=OSS%20Skills&description=Guided%20open-source%20contribution%20workflows)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx --from git+https://github.com/chiruu12/oss-skills-mcp oss-skills-mcp
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
      extensionId="oss-skills-mcp"
      extensionName="OSS Skills"
      description="Guided open-source contribution workflows - the agent researches, you think and code"
      type="stdio"
      command="uvx"
      args={["--from", "git+https://github.com/chiruu12/oss-skills-mcp", "oss-skills-mcp"]}
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="OSS Skills"
      description="Guided open-source contribution workflows - the agent researches, you think and code"
      type="stdio"
      command="uvx --from git+https://github.com/chiruu12/oss-skills-mcp oss-skills-mcp"
      timeout={300}
    />
  </TabItem>

</Tabs>

## Example Usage

The server exposes each skill as an MCP prompt, plus two tools: `list_skills` (names and descriptions of all bundled skills) and `get_skill` (full skill content).

The skills cover the whole contribution journey:

- **Evaluation**: `oss-evaluate-repo`
- **Workflow**: `oss-find-issue`, `oss-prep-to-contribute`, `oss-setup-dev-env`, `oss-contribute`, `oss-submit-pr`, `oss-post-pr`, `oss-debug-ci`
- **Contribution types**: `oss-write-tests`, `oss-write-docs`
- **Going deeper**: `oss-review-prs`, `oss-second-contribution`, `oss-find-real-issues`, `oss-explore-repo`, `oss-learn-stack`

### goose Prompt

Run the `oss-find-issue` prompt from the prompts list, or ask directly:

> Use the oss-find-issue skill to help me find a first contribution.

### goose Output

:::note CLI
Before searching for issues, I need to know who I'm matching for.

1. **What languages and frameworks are you comfortable with?**
2. **What's your experience level?** (first contribution / a few PRs merged / experienced contributor)
3. **Any specific repos or domains you're interested in?** (web, CLI tools, data, infra, etc.)

Reply with those three and I'll find unclaimed, maintainer-backed issues from repos that actually merge outside contributions.
:::

From there, goose researches the candidates and presents them with context — but picking the issue, designing the fix, and writing the code stay with you.

## Resources

- Source: [oss-skills-mcp on GitHub](https://github.com/chiruu12/oss-skills-mcp)
