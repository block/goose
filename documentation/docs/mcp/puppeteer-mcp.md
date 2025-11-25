---
title: Puppeteer Extension
description: Add Puppeteer MCP Server as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://youtube.com/embed/rms0wVGnlXA" />

Server archived

This tutorial covers how to add the [Puppeteer MCP Server](https://www.pulsemcp.com/servers/merajmehrabi-puppeteer) as a goose extension, enabling goose to interact with websites - navigating pages, filling forms, clicking buttons, taking screenshots, and executing JavaScript in a real browser environment.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-puppeteer&id=puppeteer&name=Puppeteer&description=Headless%20browser%20automation)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @modelcontextprotocol/server-puppeteer
  ```
  </TabItem>
</Tabs>
:::


## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="puppeteer"
    extensionName="Puppeteer"
    description="Headless browser automation"
    command="npx"
    args={["-y", "@modelcontextprotocol/server-puppeteer"]}
  />
</TabItem>
  <TabItem value="cli" label="goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension (SSE) 
    │  ○ Remote Extension (Streaming HTTP) 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  Puppeteer
    // highlight-end
    └ 
  ```

  4. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Puppeteer
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-puppeteer
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds goose should wait for actions to complete before timing out. Default is 300s
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Puppeteer
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-puppeteer
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    └ 
  ``` 

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Puppeteer
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-puppeteer
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    └ 
  ```

  7. Choose No when asked to add environment variables

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Puppeteer
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-puppeteer
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    ◇  Would you like to add a description?
    │  No
    │    
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No 
    │
    // highlight-end
    └  Added puppeteer extension
  ```  

  </TabItem>
</Tabs>


## Example Usage
In this example, I’ll show you how to use goose with the Puppeteer Extension to perform an accessibility audit on a website. By automating browser interactions, goose can navigate the site, test keyboard navigation, check color contrast, analyze ARIA attributes, and generate a detailed report with recommendations and code fixes.

This allows you to quickly identify and resolve accessibility issues without manually inspecting each page.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Open a new session in goose Desktop
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  1. Open a terminal and start a new goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### goose Prompt
```
Can you check if my website is accessible? Please conduct a full accessibility audit, focusing on the following:

1. Keyboard Navigation: Test all interactive elements (links, buttons, dropdowns, etc.) to ensure they are accessible without a mouse.

2. Color Contrast Analysis: Verify that all text and UI elements meet WCAG contrast ratio standards.

3. Screen Reader Compatibility: Test whether a screen reader can properly interpret content and navigation.

4. ARIA & Semantic HTML Validation: Identify any missing or misused ARIA attributes and ensure proper semantic structure.

5. Error Identification & Screenshots: Provide a comprehensive report with identified issues, along with screenshots if necessary.

6. Fixes & Code Recommendations: Suggest HTML, CSS, or JavaScript changes to resolve any compliance issues.

Here is the website to test: https://broken-workshop.dequelabs.com/. I want to ensure that our documentation meets accessibility standards. Please generate a detailed report with steps to fix any issues found.

```

### goose Output

:::note Desktop
I'll help you conduct a comprehensive accessibility audit of the website. I'll use a combination of automated and manual testing approaches to evaluate different accessibility aspects.

    1. First, let's navigate to the site
    2. Take screenshots and perform keyboard navigation testing
    3. Run accessibility audits including color contrast analysis
    4. Document findings

**Let's begin:**

    `─── Puppeteer Navigate ──────────────────────────`

**Let's take a screenshot of the homepage for reference:**

    `─── Puppeteer Screenshot ────────────────────────`


I'll conduct a detailed accessibility audit of the recipe dashboard website. From the initial view, I can already identify several accessibility concerns. Let me break down the findings by category:

**1. Keyboard Navigation Issues:**
- The "COOK" buttons for recipes appear to lack proper focus indicators
- The edit icons (pencil icons) on recipe cards don't seem to have keyboard focus states
- The navigation sequence may not be logical as there's no clear tab order

**2. Color Contrast Issues:**
- The light gray text used for "Difficulty" labels may not meet WCAG contrast requirements
- The green "Beginner" text appears to have insufficient contrast against the white background
- The bar chart in the "YUMMINESS" section uses colors that may be difficult to distinguish

**3. Semantic Structure Problems:**
- The heading hierarchy is unclear - "RECIPE DASHBOARD" appears to be an h1, but recipe titles don't seem to use proper heading tags
- The statistics (9 EGGS USED, 10 RECIPES MADE, etc.) don't appear to use semantic list structures
- The chart lacks proper ARIA labels for screen readers

**4. ARIA and Screen Reader Issues:**
- The recipe cards don't appear to have proper aria-labelledby associations
- The edit buttons lack aria-labels describing their purpose
- The chart data isn't properly accessible to screen readers
- No ARIA landmarks are visible for main navigation areas

Let me run some additional tests to verify these issues:

    `─── Puppeteer Evaluate ──────────────────────────`

Let me capture some specific sections for detailed analysis:

    `─── Puppeteer Screenshot ──────────────────────────`

Based on my analysis, here's a detailed report with recommendations:

**ACCESSIBILITY AUDIT REPORT**

**1. Structural Issues:**

```html
<!-- Current -->
<div class="recipe-title">Chocolate Cake</div>

<!-- Recommended -->
<h2 class="recipe-title">Chocolate Cake</h2>
```
**2. Interactive Elements:**

```html
<!-- Current -->
<button class="cook-button">COOK CHOCOLATE CAKE</button>

<!-- Recommended -->
<button 
  class="cook-button"
  aria-label="Cook Chocolate Cake recipe"
  tabindex="0">
  COOK CHOCOLATE CAKE
</button>
```
**3. Image Accessibility:**
```html
<!-- Current -->
<img src="chocolate-cake.jpg">

<!-- Recommended -->
<img 
  src="chocolate-cake.jpg" 
  alt="Chocolate cake with dark chocolate frosting"
  role="img">
```

**4. Chart Accessibility:**

```html
<!-- Recommended addition -->
<div class="yumminess-chart" 
     role="region" 
     aria-label="Monthly yumminess ratings">
  <table class="sr-only">
    <!-- Add hidden table for screen readers -->
  </table>
</div>
```
**5. Color Contrast Fixes:**

```css
/* Current */
.difficulty-beginner {
  color: #90EE90;
}

/* Recommended */
.difficulty-beginner {
  color: #2E7D32; /* Darker green for better contrast */
}
```
**6. Focus Management:**

```css
/* Add to CSS */
:focus {
  outline: 3px solid #2196F3;
  outline-offset: 2px;
}

.cook-button:focus {
  box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.4);
}
```
**Priority Fixes Needed:**

**1. High Priority:**

- Add proper heading structure using h1-h6 tags
- Implement keyboard focus indicators
- Add alt text to all recipe images
- Fix color contrast for difficulty labels

**2. Medium Priority:**

- Add ARIA labels to interactive elements
- Implement proper list structure for statistics
- Add skip links for keyboard navigation
- Make chart data accessible via screen reader

**3. Low Priority:**

- Enhance focus order
- Add ARIA landmarks
- Implement announcement of dynamic content changes

**Additional Recommendations:**

**1. Add a skip link:**

```html
<a href="#main-content" class="skip-link">
  Skip to main content
</a>
```
**2. Implement proper form labels:**

```html
<label for="prep-time">Preparation Time</label>
<input id="prep-time" type="text" value="20 min">
```
**3. Add ARIA live regions for dynamic updates:**

```html
<div aria-live="polite" role="status">
  <!-- Dynamic content updates -->
</div>
```

**Testing Tools Recommended:**

1. WAVE Web Accessibility Evaluation Tool
2. axe DevTools
3. Keyboard-only navigation testing
4. Screen reader testing with VoiceOver (Mac)

This audit reveals several critical accessibility issues that should be addressed to ensure WCAG 2.1 compliance. The most urgent concerns are the lack of proper semantic structure, insufficient color contrast, and missing ARIA attributes. Would you like me to provide more specific details about implementing any of these fixes?
:::

### Further Automation with GitHub Extension

You can take this accessibility audit a step further by combining the [GitHub Extension](/docs/mcp/github-mcp) with the Puppeteer Extension. With this setup, goose doesn’t just find issues and apply fixes, it can also handle the entire Git workflow for you. The GitHub Extension allows goose to commit changes, create a pull request, and even generate a PR description, so all you have to do is review and merge.

1. Enable the GitHub extension by following the steps in the **[GitHub Extension Tutorial](/docs/mcp/github-mcp#configuration)**.


:::tip 
Ensure your GitHub Personal Access Token has the necessary permissions for repository access and pull request creation when using this combined approach. 
:::

2. Ask goose to:

   - Create a new branch
   - Commit the accessibility improvements
   - Open a pull request

### goose prompt:

```
Can you create a new branch called 'accessibility-improvements', apply the accessibility fixes you suggested, and open a pull request with these changes?
```
goose will then:
   - ✅ Create a branch: `accessibility-improvements`
   - ✅ Apply the recommended accessibility fixes
   - ✅ Commit the changes with a descriptive message
   - ✅ Open a pull request with a summary of improvements
