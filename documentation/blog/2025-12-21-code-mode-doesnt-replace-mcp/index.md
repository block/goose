---
title: "Code Mode Doesn't Replace MCP (Here's What It Actually Does)"
description: Code Mode isn't killing MCP. It makes it better. A practical look at how Code Mode works with MCP to solve tool bloat and performance issues in agents.
authors:
    - rizel
---

![Code Mode and MCP working together](header-image.jpg)

One day, we will tell our kids we used to have to wait for agents, but they wouldn't know that world because the agents in their day would be so fast. I joked about this with Nick Cooper, an MCP steering committee member from OpenAI, and Bradley Axen, the creator of goose.

They both laughed at the thought because they understand exactly how clunky and experimental our current "dial-up era" of agentic workflows feels. While the Model Context Protocol (MCP) has already moved the needle by making it easier to connect our everyday apps, the experience is still far from perfect. We are still figuring out how to balance the power of these tools with the technical constraints of the models themselves.

<!-- truncate -->

---

## The "Too Many Extensions" Problem

In the goose ecosystem, we refer to MCP servers as **extensions**, and managing them effectively is one of the biggest challenges for new users. Many people write off the entire concept of AI agents because they experience lag or instability, often without realizing they have fallen into the trap of "extension bloat." The standard advice from the goose devrel team and power users is simple but often ignored: **do not turn on too many extensions at once.**

When you activate a dozen extensions for GitHub, Vercel, Slack, and your databases, you are effectively flooding the agent's context window with thousands of tokens worth of documentation. Each tool call requires the model to hold all those definitions in its "active memory," which leads to a noticeable degradation in performance. The agent becomes slower, begins to hallucinate details that aren't there, and eventually starts throwing errors, leading the frustrated user to conclude that the tool simply isn't ready for prime time.

## Making Extensions Dynamic

The goose team initially tried to combat this by adding **dynamic extensions**, which allow the system to keep most tools dormant until the agent specifically identifies a need for them. While this was a massive step forward for efficiency, it remained a somewhat hidden feature that many casual users never discovered. I spent plenty of time watching people operate with a massive list of active extensions, cringing as I realized how much of their token budget was being wasted on tools they weren't even using.

## Misconceptions I Had About Code Mode

Before I dive into how this works, I want to clear up a few misconceptions I personally held before I really understood the technology. It is easy to look at a new feature and make assumptions about its purpose, but Code Mode is more specific than I initially realized:

* **I thought it would make everything faster:** Code Mode doesn't necessarily make the clock move faster; in fact, it often requires more "round-trips" because the LLM has to discover tools and write JavaScript before it can act.
* **I thought it was for every task:** If you are only using one or two tools, the overhead of writing and executing code might actually be more work than just calling the tool directly.
* **I thought it was a replacement for MCP:** Code Mode is actually an enhancement built directly on top of MCP, much like how GitHub is built on top of Git to make the underlying technology more accessible.

The brilliance of Cloudflare's original Code Mode idea lies in its simplicity. Instead of forcing the LLM to memorize a hundred different tool definitions, you provide it with just three foundational tools: `search_modules`, `read_module`, and `execute_code`. The agent then learns to find what it needs on the fly and writes a custom script to chain those actions together in a single execution.

---

## How goose Implemented Code Mode

goose took a unique approach by making Code Mode itself an extension called the **Code Execution extension**. When this is active, it wraps your other extensions and exposes them as JavaScript modules, allowing the LLM to see only three tools instead of eighty.

When the agent needs to perform a complex task, it writes a script that looks something like this:

```javascript
import { shell, text_editor } from "developer";

const branch = shell({ command: "git branch --show-current" });
const commits = shell({ command: "git log -3 --oneline" });
const packageJson = text_editor({ path: "package.json", command: "view" });
const version = JSON.parse(packageJson).version;

text_editor({ 
  path: "LOG.md", 
  command: "write", 
  file_text: `# Log\n\nBranch: ${branch}\n\nCommits:\n${commits}\n\nVersion: ${version}` 
});
```

## Putting Code Mode to the Test

I knew I had to put these claims to the test myself to see if the efficiency gains were as significant as the technical specs suggested. For this experiment, I was using **Claude Opus 4.5**, enabled eight different extensions, and gave the agent a single, multi-step prompt to see how it handled the load:

> "Create a LOG.md file with the current git branch, last 3 commits, and the version from package.json"

### The Standard MCP Approach

When I ran this test with Code Mode disabled, the agent successfully performed five separate tool calls to gather the data and write the file. However, because all eight extensions had their full definitions loaded into the context, this relatively simple task consumed **16% of my total context window**. This demonstrates the clear scalability issues of standard workflows, as the system becomes increasingly unstable and prone to failure when you aren't using Code Mode.

### The Code Mode Advantage

When I toggled Code Mode on and ran the exact same prompt, the experience changed completely. The agent used its discovery tools to find the necessary modules and wrote a single, unified JavaScript script to handle the entire workflow at once. In this scenario, **only 3% of the context window was used.**

The results were undeniable: the same task, with the same extensions available, was five times more efficient on tokens. This provides the breathing room needed for much longer sessions before the model's performance begins to degrade or it begins to hallucinate under the weight of too many tools.

---

## What's Next for goose

The team is currently refining Code Mode following its release in [goose v1.17.0](/blog/2025/12/15/code-mode-mcp-in-goose) earlier this month. We are focusing on improving the user experience by showing which tools are being called rather than just displaying raw JavaScript, and we are working on better type signatures to ensure the LLM gets the code right the first time.

Code Mode is a massive step forward in building agents that can scale to handle all your tools without falling apart. I love seeing how MCP is evolving, and I can't wait for the day I'm telling my children that agents weren't always this limitless and that we actually used to have to ration our tools just to get a simple task done.

<head>
  <meta property="og:title" content="Code Mode Doesn't Replace MCP (Here's What It Actually Does)" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2025/12/21/code-mode-doesnt-replace-mcp" />
  <meta property="og:description" content="Code Mode isn't killing MCP. It makes it better. A practical look at how Code Mode works with MCP to solve tool bloat and performance issues in agents." />
  <meta property="og:image" content="https://block.github.io/goose/assets/images/header-image.jpg" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="Code Mode Doesn't Replace MCP (Here's What It Actually Does)" />
  <meta name="twitter:description" content="Code Mode isn't killing MCP. It makes it better. A practical look at how Code Mode works with MCP to solve tool bloat and performance issues in agents." />
  <meta name="twitter:image" content="https://block.github.io/goose/assets/images/header-image.jpg" />
</head>
