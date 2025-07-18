---
title: JetBrains Extension
description: Use JetBrains MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/1fP5elf9qQM" />

The JetBrains extension is designed to work within your IDE. Goose can accomplish a lot of the developer-centric tasks with the Developer extension that is enabled on install, however, the JetBrains extension provides a more integrated and project-aware way to work with code.

This tutorial covers how to enable and use the JetBrains MCP Server as a built-in Goose extension to integrate with any JetBrains IDE.

## Configuration

1. Add the [MCP Server plugin](https://plugins.jetbrains.com/plugin/26071-mcp-server) to your IDE.

2. Enable built-in Goose extension:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Jetbrains"
    description="Integrate Goose with any JetBrains IDE"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Built-in Extension`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◆  What type of extension would you like to add?
  // highlight-start    
  │  ● Built-in Extension (Use an extension that comes with Goose)
  // highlight-end  
  │  ○ Command-line Extension 
  │  ○ Remote Extension (SSE) 
  │  ○ Remote Extension (Streaming HTTP) 
  └  
  ```

  3. Arrow down to the `JetBrains` extension and press Enter
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◆  Which built-in extension would you like to enable?
  │  ○ Developer Tools 
  │  ○ Computer Controller 
  │  ○ Google Drive 
  │  ○ Memory 
  // highlight-start
  │  ● JetBrains (Connect to jetbrains IDEs)
  // highlight-end
  └
  ```

  4. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s

  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◇  Which built-in extension would you like to enable?
  │  JetBrains
  │
  // highlight-start
  ◆  Please set the timeout for this tool (in secs):
  │  300
  // highlight-end
  │
  └  Enabled jetbrains extension
  ```

  5. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◇  Which built-in extension would you like to enable?
  │  JetBrains
  │
  ◇  Please set the timeout for this tool (in secs):
  │  300
  │
  // highlight-start
  ◆  Would you like to add a description?
  │  No
  // highlight-end
  │
  └
  ```
  </TabItem>
</Tabs>

## Example Usage

In this example, I'm going to upgrade a Java project to the latest LTS version.

:::info LLM
Anthropic's Claude 3.5 Sonnet was used for this task.
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
   1. Open [IntelliJ](https://www.jetbrains.com/idea/download) (JetBrains' Java and Kotlin IDE)
   2. Open a new session in Goose Desktop
   :::note
   You will interact with two separate apps: the Goose Desktop app and the IntelliJ IDE.
   :::

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Open [IntelliJ](https://www.jetbrains.com/idea/download) (JetBrains' Java and Kotlin IDE)
  2. Open a terminal within your IDE and start a new Goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

:::tip 
When using Goose to refactor code, it's recommended to commit your code to version control _before_ making changes. This way, you have a clean snapshot to compare against, making it easy to review Goose’s modifications, revert if needed, and track improvements.
:::

### Goose Prompt
```
using the JetBrains IDE tools, upgrade this project to use the latest Java LTS version and refactor the codebase to use newer Java features where beneficial.
```

After 2 minutes, Goose was done with the upgrade! 🎉

### Goose Output

```md
I'll make several improvements to modernize this codebase to use Java 21 (latest LTS) features. Here are the changes I'll implement:

1. Update the `pom.xml` to use Java 21
2. Use newer Java features 

[... code changes ...]

Here's a summary of the modernization changes made:

1. `pom.xml`:
   - Set Java version to 21
   - Enabled preview features
   - Updated all dependencies to their latest stable versions

2. `AccountsOverviewPage.java`:
   - Made the class `final` since it extends the sealed `Page` class
   - Updated stream collection to use `.toList()` instead of `collect(Collectors.toList())`
   - Made fields final for better immutability

3. `Page.java`:
   - Converted to a sealed class
   - Restricted which classes can extend it using `permits`
   - Made fields protected and final where appropriate
   - Added pattern matching for exception handling
   - Implemented Optional for null safety
   - Used String Templates for error messages
   - Added functional programming concepts with Supplier
   - Enhanced error handling with specific exception types

4. `LoginComponent.java`:
   - Added a private record for credentials
   - Implemented pattern matching with record patterns
   - Made fields final for better immutability

5. `AccountTests.java`:
   - Implemented structured concurrency using `StructuredTaskScope`
   - Used parallel execution for UI and API tasks
   - Improved error handling with structured task management
   - Used pattern matching and type inference with `var`

6. `BaseTests.java`:
   - Implemented structured concurrency for setup tasks
   - Improved resource management
   - Better exception handling
   - Used modern Duration API

7. `APIUtil.java`:
   - Implemented String Templates (Java 21 preview feature)
   - Used modern text blocks

8. `Customers.java`:
   - Added sealed interface for customer types
   - Implemented pattern matching for switch expressions
   - Added pattern matching for instanceof
   - Enhanced type safety with records

Key modern Java features used:
- String Templates (Java 21 Preview)
- Pattern Matching for switch (Java 21)
- Sealed Classes and Interfaces (Java 17)
- Records (Java 16)
- Pattern Matching for instanceof (Java 16)
- Text Blocks (Java 15)
- Switch Expressions (Java 14)

The codebase is now using the latest Java LTS version (21) and incorporates many modern Java features that improve:
- Type safety
- Null safety
- Code readability
- Error handling
- Performance
```





Using the JetBrains extension provided several advantages and made it easier for Goose to:
- navigate through the codebase
- track changes
- make consistent modifications across files
- verify the content of files before and after changes

