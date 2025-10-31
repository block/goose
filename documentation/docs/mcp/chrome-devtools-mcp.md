---
title: Chrome DevTools Extension
description: Add Chrome DevTools MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the Chrome DevTools MCP Server as a Goose extension to enable browser automation, web performance testing, and interactive web application debugging in a Chrome browser.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=chrome-devtools-mcp%40latest&id=chrome-devtools&name=Chrome%20DevTools&description=Browser%20automation%20and%20web%20performance%20testing%20capabilities)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y chrome-devtools-mcp@latest
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
      extensionId="chrome-devtools"
      extensionName="Chrome DevTools"
      description="Browser automation and web performance testing capabilities"
      command="npx"
      args={["-y", "chrome-devtools-mcp@latest"]}
      cliCommand="npx -y chrome-devtools-mcp@latest"
      timeout={300}
      note="Note that you'll need Node.js installed on your system to run this command, as it uses npx."
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
    │  chrome-devtools
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
    │  chrome-devtools
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y chrome-devtools-mcp@latest
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
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
    │  chrome-devtools
    │
    ◇  What command should be run?
    │  npx -y chrome-devtools-mcp@latest
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
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
    │  chrome-devtools
    │
    ◇  What command should be run?
    │  npx -y chrome-devtools-mcp@latest
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    │
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
    │  chrome-devtools
    │
    ◇  What command should be run?
    │  npx -y chrome-devtools-mcp@latest
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
    |
    // highlight-end
    └  Added chrome-devtools extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

In this example, I'll show you how to use Goose with the Chrome DevTools Extension to perform automated performance testing on a demo website.

### Goose Prompt
```
Use the chrome dev tools extension to Navigate to https://www.saucedemo.com 
Then, login as performance_glitch_user and measure the site's performance
```
### Goose Output

```

I'll help you navigate to the Sauce Demo site, login as the performance_glitch_user, and measure the performance impact. Let me start by checking what pages are currently open and then navigate to the site.

Perfect! I can see the login page for Swag Labs. I can see that `performance_glitch_user` is one of the accepted usernames, and the password for all users is `secret_sauce`. Let me log in with these credentials and then measure the performance.

I've successfully navigated to the Saucedemo website and logged in as the `performance_glitch_user` to measure the site's performance. Here's what I discovered:

## Performance Analysis Results for performance_glitch_user

### Initial Observations

1. **Login Performance**: The initial login with the performance_glitch_user took noticeably longer than normal - the click on the Login button actually timed out after 5 seconds, indicating significant delays.

2. **Page Load Metrics**: 
   - **Total Load Time**: ~5.03 seconds (5030ms)
   - **First Paint**: 5.08 seconds 
   - **First Contentful Paint**: 5.08 seconds
   - **Response Time**: 2.5ms (server response was actually fast)
   - **DOM Content Loaded**: 0.1ms

### Performance Issues Identified

The performance issues with the `performance_glitch_user` account appear to be **client-side rendering delays** rather than network or server issues:

- The server responds quickly (2.5ms response time)
- The major delay occurs in the rendering phase (5+ seconds to first paint)
- This suggests the performance glitch is intentionally built into the client-side JavaScript execution for this specific user

### Interactive Performance

- **Button Clicks**: Adding items to cart works, but with noticeable delays
- **DOM Queries**: Standard DOM operations perform normally (~0.1ms)
- **JavaScript Execution**: Basic computations run at normal speed (~2ms for 100k operations)

### Cart Functionality

I was able to successfully add multiple items to the cart (the cart counter increased from 1 to 3), showing that despite the performance issues, the core functionality remains intact.

### Conclusion

The `performance_glitch_user` account demonstrates **intentional performance degradation** primarily affecting:
- Initial page rendering (5+ second delays)
- Login process (timeouts)
- Visual feedback delays

This makes it an excellent test account for performance testing scenarios, as it simulates real-world performance issues that users might experience with slow devices or poor network conditions, while maintaining functional correctness.
```