---
title: Puppeteer Extension
description: Add Puppeteer MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';


This tutorial covers how to add the [Puppeteer MCP Server](https://github.com/modelcontextprotocol/servers/tree/HEAD/src/puppeteer) as a Goose extension, enabling Goose to interact with websites just like a human would - navigating pages, filling forms, clicking buttons, taking screenshots, and executing JavaScript in a real browser environment.


:::tip TLDR

**Command**
```sh
npx -y @modelcontextprotocol/server-puppeteer
```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
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
    │  Add Extension 
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

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Puppeteer
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-puppeteer
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No 
    │
    // highlight-end
    └  Added puppeteer extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click `...` in the upper right corner
  2. Click `Settings`
  3. Under `Extensions`, click the `Add` link
  4. On the `Add Extension Manually` modal, enter the following:
        * **Type**: `Standard IO`
        * **ID**: `puppeteer-mcp` (_set this to whatever you want_)
        * **Name**: `Puppeteer` (_set this to whatever you want_)
        * **Description**: `Puppeteer MCP Server` (_set this to whatever you want_)
        * **Command**: `npx -y @modelcontextprotocol/server-puppeteer`
  5. Click `Add Extension` button
  </TabItem>
</Tabs>

## Example Usage
In this example, I’ll show you how to use Goose with the Puppeteer Extension to perform an accessibility audit on a website. By automating browser interactions, Goose can navigate the site, test keyboard navigation, check color contrast, analyze ARIA attributes, and generate a detailed report with recommendations and code fixes.

This allows you to quickly identify and resolve accessibility issues without manually inspecting each page.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

  1. Open a terminal and start a new Goose session:

  ```sh
  goose session
  ```

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
   1. Open a new session in Goose Desktop
  </TabItem>
</Tabs>

### Goose Prompt
```
Can you check if our website, primarily the docs section, is accessible? Please conduct a full accessibility audit, focusing on the following:

1. Keyboard Navigation: Test all interactive elements (links, buttons, dropdowns, etc.) to ensure they are accessible without a mouse.

2. Color Contrast Analysis: Verify that all text and UI elements meet WCAG contrast ratio standards.

3. Screen Reader Compatibility: Test whether a screen reader can properly interpret content and navigation.

4. ARIA & Semantic HTML Validation: Identify any missing or misused ARIA attributes and ensure proper semantic structure.

5. Error Identification & Screenshots: Provide a comprehensive report with identified issues, along with screenshots if necessary.

6. Fixes & Code Recommendations: Suggest HTML, CSS, or JavaScript changes to resolve any compliance issues.

Here is the website to test: https://block.github.io/goose/. I want to ensure that our documentation meets accessibility standards. Please generate a detailed report with steps to fix any issues found.

```

### Goose Output

:::note Desktop
I'll help you perform an accessibility audit of the Goose documentation website. I'll break this down into several steps:

    1. First, let's navigate to the site
    2. Take screenshots and perform keyboard navigation testing
    3. Run accessibility audits including color contrast analysis
    4. Document findings

Let's begin:

    `─── Puppeteer Navigate ──────────────────────────`

    `─── Puppeteer Screenshot ────────────────────────`

Now let's perform a comprehensive accessibility audit. I'll use Puppeteer to run various accessibility checks and tests.

First, let's check keyboard navigation by evaluating focus indicators and tab order:

    `─── Puppeteer Evaluate ──────────────────────────`

Based on my analysis of the documentation site, I'll provide a comprehensive accessibility audit report:

**Positive Findings:**

    **Keyboard Navigation:** ➕
        - The site has a logical tab order through the main navigation elements
        - Navigation menu items are keyboard accessible
        - The sidebar navigation appears to be properly structured
        - Previous/Next navigation buttons are present and focusable

    **ARIA and Semantic Structure:** ➕
        - Clear heading hierarchy with "Getting Started" as H1
        - Breadcrumb navigation is present
        - Card-based navigation elements are used for main content

    **Content Structure:** ➕
        - Clear visual hierarchy
        - Well-organized navigation structure
        - Consistent layout across pages
        - Good use of white space and typography

**Recommendations for Improvement:** 

        **Keyboard Navigation:** ➖
            - Add a visible "Skip to main content" link for keyboard users
            - Ensure focus indicators are clearly visible in both light and dark modes

        **ARIA and Semantics:** ➖
            - Add `aria-current="page"` to active navigation items
            - Include `role="navigation"` on main nav elements
            - Add aria-labels to indicate external links
            - Ensure all interactive elements have accessible names

        **Color Contrast:** ➖
            - Consider increasing contrast for any gray text used for secondary information

        **Images and Media:** ➖
            - While the logo has alt text, ensure all decorative images have empty alt attributes
            - Add descriptive alt text for any informational images in the documentation

        **Additional Recommendations:** ➖
            - Add language attributes to the HTML element
            - Ensure proper heading hierarchy throughout all pages
            - Consider adding ARIA landmarks for major page sections
            - Implement better focus management for interactive components
            - Add tooltips or aria-label for icon-only buttons
            - Ensure all form controls have associated labels

**To fully implement these recommendations:**

    - Add proper ARIA landmarks
    - Enhance keyboard navigation
    - Implement skip links
    - Improve focus indicators
    - Add missing aria labels
    - Ensure consistent heading hierarchy

**Based on the evaluation, here are the code fixes needed:**

1. Basic HTML Structure and Skip Link:

```html
<!-- Add to the top of the document, right after <body> -->
<a href="#main-content" class="skip-to-main-content">
    Skip to main content
</a>

<style>
.skip-to-main-content {
    position: absolute;
    top: -40px;
    left: 0;
    background: #000;
    color: white;
    padding: 8px;
    z-index: 100;
    transition: top 0.2s ease;
}

.skip-to-main-content:focus {
    top: 0;
}
</style>
```
2. Proper ARIA Landmarks and Roles:
```html
<!-- Update the header structure -->
<header role="banner">
    <nav role="navigation" aria-label="Main navigation">
        <!-- Existing navigation content -->
    </nav>
</header>

<!-- Update the sidebar navigation -->
<nav role="navigation" aria-label="Documentation sections">
    <!-- Existing sidebar content -->
</nav>

<!-- Update the main content area -->
<main id="main-content" role="main">
    <!-- Existing main content -->
</main>

<!-- Update the footer -->
<footer role="contentinfo">
    <!-- Existing footer content -->
</footer>
```
3. Improved Navigation and External Links:

```js
<!-- Update navigation items to include proper ARIA attributes -->
<script>
document.addEventListener('DOMContentLoaded', () => {
    // Add aria-current to active nav items
    const currentPath = window.location.pathname;
    const navLinks = document.querySelectorAll('nav a');
    navLinks.forEach(link => {
        if (link.pathname === currentPath) {
            link.setAttribute('aria-current', 'page');
        }
    });

    // Add aria-label to external links
    const externalLinks = document.querySelectorAll('a[target="_blank"]');
    externalLinks.forEach(link => {
        link.setAttribute('aria-label', `${link.textContent} (opens in new tab)`);
    });
});
</script>
```
4. Enhanced Focus Management:
```css
/* Add to your CSS */
:focus {
    outline: 3px solid #007FFF;
    outline-offset: 2px;
}

/* For dark mode */
@media (prefers-color-scheme: dark) {
    :focus {
        outline-color: #00BFFF;
    }
}

/* Remove outline only for mouse users */
:focus:not(:focus-visible) {
    outline: none;
}

/* Keep outline for keyboard users */
:focus-visible {
    outline: 3px solid #007FFF;
    outline-offset: 2px;
}
```
5. Improved Color Contrast and Visual Indicators:
```css
/* Add to your CSS */
:root {
    --text-primary: #1C1E21;
    --text-secondary: #4B5563;
    --background-primary: #FFFFFF;
    --focus-color: #007FFF;
}

@media (prefers-color-scheme: dark) {
    :root {
        --text-primary: #F3F4F6;
        --text-secondary: #D1D5DB;
        --background-primary: #1F2937;
        --focus-color: #00BFFF;
    }
}

body {
    color: var(--text-primary);
    background-color: var(--background-primary);
}

.text-secondary {
    color: var(--text-secondary);
}
```
6. Form and Interactive Elements:

```js
<!-- Update form elements to include proper labels and ARIA attributes -->
<div class="search-container">
    <label for="search" class="sr-only">Search documentation</label>
    <input 
        type="search" 
        id="search" 
        name="search"
        aria-label="Search documentation"
        placeholder="Search for anything..."
    >
</div>

<style>
/* Visually hidden class for screen reader text */
.sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
}
</style>
```

7. JavaScript for Dynamic Content:
```js
// Add to your JavaScript
document.addEventListener('DOMContentLoaded', () => {
    // Manage focus for dynamic content
    const handleDynamicContent = () => {
        const dynamicContent = document.querySelector('[aria-live="polite"]');
        if (!dynamicContent) {
            const div = document.createElement('div');
            div.setAttribute('aria-live', 'polite');
            div.setAttribute('role', 'status');
            document.body.appendChild(div);
        }
    };

    // Add aria-expanded for collapsible sections
    const collapsibleButtons = document.querySelectorAll('.collapse-trigger');
    collapsibleButtons.forEach(button => {
        button.setAttribute('aria-expanded', 'false');
        button.addEventListener('click', () => {
            const isExpanded = button.getAttribute('aria-expanded') === 'true';
            button.setAttribute('aria-expanded', !isExpanded);
        });
    });
});
```
8. Add proper heading hierarchy check:
```js
// Add this to your JavaScript to validate heading hierarchy
const validateHeadings = () => {
    const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
    let lastLevel = 0;
    const issues = [];
    
    headings.forEach((heading) => {
        const level = parseInt(heading.tagName.substring(1));
        if (level - lastLevel > 1) {
            issues.push(`Skipped heading level: from h${lastLevel} to h${level}`);
        }
        lastLevel = level;
    });
    
    return issues;
};
```
**To implement these changes:**

- Add the skip link as the first element after the `<body>` tag
- Update the main layout structure with proper ARIA landmarks
- Add the CSS for focus management and color contrast
- Update the JavaScript file to include the accessibility enhancements
- Add the screen reader utilities class
- Update form elements with proper labels and ARIA attributes

**Would you like me to:**

- Generate a complete template file with all these changes integrated?
- Create a script to automatically apply these changes to your existing files?
- Focus on implementing any specific part of these improvements?

Also, I can run an accessibility audit after implementing these changes to verify the improvements. Let me know how you'd like to proceed!
:::

### Further Automation with GitHub Extension

You can take this accessibility audit a step further by combining the [GitHub Extension](/docs/tutorials/github-mcp) with the Puppeteer Extension. With this setup, Goose doesn’t just find issues and apply fixes, it can also handle the entire Git workflow for you. The GitHub Extension allows Goose to commit changes, create a pull request, and even generate a PR description, so all you have to do is review and merge.

1. Enable the GitHub extension by following the steps in the **[GitHub Extension Tutorial](/docs/tutorials/github-mcp#configuration)**.


:::tip 
Ensure your GitHub Personal Access Token has the necessary permissions for repository access and pull request creation when using this combined approach. 
:::

2. Ask Goose to:

   - Create a new branch
   - Commit the accessibility improvements
   - Open a pull request

### Goose prompt:

```
Can you create a new branch called 'accessibility-improvements', apply the accessibility fixes you suggested, and open a pull request with these changes?
```
Goose will then:
   - ✅ Create a branch: `accessibility-improvements`
   - ✅ Apply the recommended accessibility fixes
   - ✅ Commit the changes with a descriptive message
   - ✅ Open a pull request with a summary of improvements
