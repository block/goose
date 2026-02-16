---
title: Agentic Testing with Playwright Skills
description: Use goose with Playwright CLI skills to automate browsers and generate tests using natural language
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

<iframe width="560" height="315" src="https://www.youtube.com/embed/_MpbmD_unnU?si=dpHvuLVkbONN_0Hk" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

goose can automate browsers and generate Playwright tests using [Playwright CLI](https://github.com/microsoft/playwright-cli). By loading Playwright CLI Skills, goose gains the ability to navigate websites, interact with elements, take screenshots, record videos, and generate test code all from natural language instructions whilst also saving on tokens compared to when using the Playwright MCP.

### Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- A project with [Playwright](https://playwright.dev/) installed (`npm init playwright@latest`)
- Install Playwright CLI globally:
  ```bash
  npm install -g @playwright/cli@latest
  ```

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

Unlike with the Playwright MCP whcih puts the entire page structure into the LLM context, Playwright CLI saves the accessibility tree locally as YAML files. This means:

- ✅ Faster responses
- ✅ Lower token usage
- ✅ More cost effective
- ✅ Handles large pages without issues

When goose takes a snapshot, it saves a YAML file containing element references (refs) that it uses to find and interact with elements on the page.

## Example Usage

### Opening a Browser and Taking Screenshots

Ask goose to open a website:

```
Open block.github.io/goose in the browser
```

goose will navigate to the site and report the browser session is active. You can then:

```
Take a screenshot
```

The screenshot is saved to `.playwright-cli/` in your project.

:::tip Headed Mode
By default, the browser runs headless (no visible window). If you want to watch the automation, use:
```
Open block.github.io/goose in a headed browser
```

:::

### Generating Tests with Natural Language

Describe what you want to test in plain English:

```
Search for a guide on how to use skills, click on the first result, 
then create a test based on your interactions
```

### goose Output

```
─── load_skill | skills ───────────────────────────────────────
name: playwright-cli

─── playwright-cli open | developer ───────────────────────────
Opening browser and navigating to block.github.io/goose

─── playwright-cli snapshot | developer ───────────────────────
Captured page structure, found search button at ref e89

─── playwright-cli click | developer ──────────────────────────
Clicking on search button (ref e89)

─── playwright-cli fill | developer ────────────────────────────
Filling search box with "how to use skills"

─── playwright-cli snapshot | developer ───────────────────────
Found search results, first result at ref e156

─── playwright-cli click | developer ──────────────────────────
Clicking on "Using Skills" result (ref e156)

─── playwright-cli snapshot | developer ───────────────────────
Captured page after navigation to Using Skills guide

─── text_editor | developer ───────────────────────────────────
writing tests/search-skills-guide.spec.ts

✅ Test Generated Successfully
Location: tests/search-skills-guide.spec.ts
```

The generated test might look like:

```typescript
import { test, expect } from '@playwright/test';

test('search for skills guide and navigate to it', async ({ page }) => {
  await page.goto('https://block.github.io/goose');
  await expect(page).toHaveTitle(/Goose/);
  
  // Open search
  await page.getByRole('button', { name: 'Search' }).click();
  
  // Search for skills guide
  await page.getByRole('searchbox').fill('how to use skills');
  
  // Click first result
  await page.getByRole('link', { name: 'Using Skills' }).first().click();
  
  // Verify navigation
  await expect(page).toHaveURL(/using-skills/);
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Using Skills');
});
```

### Running Tests

After generating a test, run it immediately:

```
Run the test to verify it works
```

Or run manually:

```bash
npx playwright test
```

## Recording Videos and Traces

For debugging or documentation, ask goose to record the session:

```
Create a video and trace for the actions we just performed
```

### Video Recording

Videos are saved to `.playwright-cli/videos/` and are useful for:
- Debugging test failures
- Verifying agent behavior
- Including in pull requests
- Creating demos

### Viewing Traces

Playwright traces provide a detailed timeline of everything that happened. To view a trace:

```
Open the trace in the trace viewer
```

The trace viewer shows:
- Timeline of all actions
- Screenshots before/after each action
- Console logs and errors
- Network requests
- Element locators used

## Full Capabilities

Ask goose what else it can do:

```
Playwright CLI, what else can you do?
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

## Tips and Best Practices

1. **Start simple** - Begin with basic navigation and screenshots before complex test generation
2. **Use headed mode for debugging** - Add `--headed` when you need to see what's happening
3. **Review generated tests** - Always review and refine the generated test code
4. **Leverage traces** - Use traces to understand exactly what the agent did
5. **Iterate quickly** - Run tests immediately after generation to catch issues early

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright CLI GitHub](https://github.com/microsoft/playwright-cli)
- [Using Skills Guide](/docs/guides/context-engineering/using-skills) - Learn how to create and use skills with goose
- [Skills Extension Documentation](/docs/mcp/skills-mcp)
