---
title: Agentic Testing with Playwright Skills
description: Use goose with Playwright CLI skills to automate browsers and generate tests using natural language
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

<iframe width="560" height="315" src="https://www.youtube.com/embed/_MpbmD_unnU?si=dpHvuLVkbONN_0Hk" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

goose can automate browsers and generate Playwright tests using [Playwright CLI](https://github.com/microsoft/playwright-cli). By loading Playwright CLI Skills, goose gains the ability to navigate websites, interact with elements, take screenshots, record videos, and generate test code—all from natural language instructions while saving on tokens compared to using the Playwright MCP.

## Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- Install Playwright CLI globally:
  ```bash
  npm install -g @playwright/cli@latest
  ```
- (Optional) [Playwright](https://playwright.dev/) installed if you want to run the generated tests (`npm init playwright@latest`)

## Configuration

First, install the Playwright skills in your project directory:

```bash
playwright-cli install --skills
```

This creates a `.claude/` folder with skills and reference files that teach goose how to use Playwright CLI capabilities.

Then, enable the [Skills extension](/docs/mcp/skills-mcp) to allow goose to load and use Agent Skills.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Skills"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  Enable extensions: (use "space" to toggle and "enter" to submit)
  // highlight-start    
  │  ● skills
  // highlight-end
  |
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Understanding the Skills Structure

After installing skills, your project will have:

```
.claude/
└── skills/
    └── playwright-cli/
        ├── SKILL.md
        └── references/
            ├── request-mocking.md
            ├── running-code.md
            ├── session-management.md
            ├── storage-state.md
            ├── test-generation.md
            ├── tracing.md
            └── video-recording.md
```

These reference files teach goose how to:
- **Mock requests** - Intercept, mock, modify, and block network requests
- **Run code** - Execute Playwright code for advanced scenarios
- **Manage sessions** - Handle browser sessions and state
- **Store state** - Manage authentication and cookies
- **Generate tests** - Create Playwright TypeScript code from actions
- **Record traces** - Capture detailed traces of browser interactions
- **Record videos** - Save browser sessions as video for debugging

## How It Works: Token Efficiency

Unlike the Playwright MCP which puts the entire page structure into the LLM context, Playwright CLI saves the accessibility tree locally as YAML files. This means:

- ✅ Faster responses
- ✅ Lower token usage
- ✅ More cost effective
- ✅ Handles large pages without issues

When goose takes a snapshot, it saves a YAML file containing element references (refs) that it uses to find and interact with elements on the page.

## Generate a Test with Video and Traces

Give goose a single prompt that describes what you want to test:

```
Open block.github.io/goose, click on the Docs menu, click on Context Engineering, 
then click on Using Skills and generate a test with video and traces
```

### How It Works

Each `playwright-cli` command automatically outputs the corresponding Playwright code. For example:

```bash
playwright-cli click e11
# Ran Playwright code:
# await page.getByRole('link', { name: 'Docs' }).click();
```

goose collects these code snippets as it performs actions and assembles them into a complete test file.

### What goose Does

1. Opens the browser: `playwright-cli open block.github.io/goose`
2. Starts recording: `playwright-cli video-start` and `playwright-cli tracing-start`
3. Takes snapshots to find elements: `playwright-cli snapshot`
4. Performs clicks: `playwright-cli click <ref>`
5. Stops recording: `playwright-cli video-stop` and `playwright-cli tracing-stop`
6. Assembles the generated code into a test file

### Generated Files

| File | Description |
|------|-------------|
| `tests/using-skills-navigation.spec.ts` | Your Playwright test |
| `.playwright-cli/video-*.webm` | Video recording of the session |
| `.playwright-cli/traces/*.trace` | Trace file for debugging |

### Generated Test Code

The generated test might look like:

```typescript
import { test, expect } from '@playwright/test';

test('navigate to Using Skills guide via docs menu', async ({ page }) => {
  await page.goto('https://block.github.io/goose');
  await expect(page).toHaveTitle(/goose/);
  
  // Click on Docs in the navigation
  await page.getByRole('link', { name: 'Docs' }).click();
  
  // Expand Context Engineering category
  await page.getByRole('button', { name: 'Expand sidebar category \'Context Engineering\'' }).click();
  
  // Click on Using Skills
  await page.getByRole('link', { name: 'Using Skills' }).click();
  
  // Verify navigation
  await expect(page).toHaveURL(/using-skills/);
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Using Skills');
});
```

### Running the Test

goose will ask if you want to run the test. If Playwright is already set up, it runs immediately. If not, goose can install Playwright for you first.

### Viewing the Trace

To debug or review what happened, ask goose:

```
Open the trace
```

The trace viewer shows:
- Timeline of all actions
- Screenshots before/after each action
- Console logs and errors
- Network requests
- Element locators used

:::tip Headed Mode
By default, the browser runs headless (no visible window). If you want to watch the automation in real-time:
```
Open block.github.io/goose in a headed browser
```
:::

## Full Capabilities

Want to know what else you can do? Ask goose:

```
What else can you do with playwright skills?
```

| Category | Capabilities |
|----------|-------------|
| **Browser Control** | open, goto, click, fill, close |
| **Capture & Debug** | screenshot, snapshot, video, trace |
| **Tab Management** | Open, switch, close tabs |
| **Storage & Auth** | Save/restore cookies, handle login states |
| **Network** | Mock APIs, intercept requests |
| **Input** | Type text, press keys, mouse actions |
| **Monitoring** | show (visual dashboard to observe all sessions) |

### Example Use Cases

- ✅ Test web applications with natural language
- ✅ Fill out forms automatically
- ✅ Scrape data from websites
- ✅ Debug issues with video recordings
- ✅ Test authentication flows
- ✅ Record demos for documentation
- ✅ Mock APIs for isolated testing

## Tips and Best Practices

1. **Start simple** - Begin with basic navigation before complex test generation
2. **Use headed mode for debugging** - Watch what's happening in real-time
3. **Review generated tests** - Always review and refine the generated test code
4. **Leverage traces** - Use traces to understand exactly what the agent did
5. **Iterate quickly** - Run tests immediately after generation to catch issues early

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright CLI GitHub](https://github.com/microsoft/playwright-cli)
- [Using Skills Guide](/docs/guides/context-engineering/using-skills) - Learn how to create and use skills with goose
- [Skills Extension Documentation](/docs/mcp/skills-mcp)
