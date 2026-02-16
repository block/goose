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

goose can even run the test for you to make sure it works as expected. If Playwright is already set up, just ask it to run the test. If not, goose can install Playwright for you and then run the test.

## Viewing the Video

To see a video of what happened, ask goose:

```
Show me the video
```

Goose will open the recorded video so you can see exactly what happened during the session.

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



## Visual Dashboard for Multiple Sessions

When you have goose running several browser tasks at once, it can be hard to keep track of what's happening. The visual dashboard gives you a bird's-eye view of all your active browser sessions, letting you watch progress in real-time or jump in and take control when needed.

```bash
playwright-cli show
```

From here you can see live previews of every browser goose is controlling. Click into any session to watch it full-size, or take over the mouse and keyboard yourself if goose needs a hand. Press **Escape** when you're done and goose picks up right where you left off.

:::note
The `show` command requires playwright-cli version 0.2.0 or later.
:::

## Full Capabilities

Want to know what else the Playwright skills can do? Ask goose:

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


### Example Use Cases

- ✅ Test web applications with natural language
- ✅ Fill out forms automatically
- ✅ Scrape data from websites
- ✅ Debug issues with video recordings
- ✅ Test authentication flows
- ✅ Record demos for documentation
- ✅ Mock APIs for isolated testing

## Conclusion

Getting started with Playwright skills is easy and opens up powerful browser automation capabilities directly from natural language prompts. Whether you're generating tests, debugging with videos and traces, or automating complex interactions, the Playwright CLI skills provide a token-efficient way to leverage Playwright's full power with goose.

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright CLI GitHub](https://github.com/microsoft/playwright-cli)
- [Using Skills Guide](/docs/guides/context-engineering/using-skills) - Learn how to create and use skills with goose
- [Skills Extension Documentation](/docs/mcp/skills-mcp)
