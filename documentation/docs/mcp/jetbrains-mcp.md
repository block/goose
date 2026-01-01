---
title: JetBrains Extension
description: Use JetBrains MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/1fP5elf9qQM" />

This tutorial covers how to add the JetBrains extension to integrate with any JetBrains IDE. While goose can use the [Developer extension](/docs/mcp/developer-mcp) for developer-centric tasks, the JetBrains extension provides a more integrated and project-aware way to work with code.

## Configuration

**Important**: The configuration steps depend on your IDE version. You can find the version in `[IDE Name] > About` (macOS) or `Help > About` (Windows/Linux).

<Tabs groupId="ideVersion">
  <TabItem value="later" label="2025.2 and later" default>

    Versions 2025.2 and later have built-in MCP server support and generate a dynamic configuration specific to your IDE instance.
    
    The instructions in this tutorial show how to configure the recommended remote SSE extension. See your IDE's documentation for more details (e.g. [MCP Server](https://www.jetbrains.com/help/idea/mcp-server.html) for IntelliJ IDEA).

    :::tip TLDR
    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
      Use `Add custom extension` in Settings → Extensions to add a `Server-Sent Events (SSE)` extension type with your IDE-specific SSE config.
      </TabItem>
      <TabItem value="cli" label="goose CLI">
      Use `goose configure` to add a `Remote Extension (SSE)` extension type with your IDE-specific SSE config.
      </TabItem>
    </Tabs>
    :::

    <br/>
    Configure the extension using your IDE's built-in MCP server support:

    1. Get your IDE-specific config:

       1. Go to `Settings > Tools > MCP Server` in your IDE
       2. If needed, click `Enable MCP Server` to enable the MCP server
       3. Click `Copy SSE Config`
       4. Click `OK` to save your changes and start the server
       5. Copy the `url` value from the config

    2. Add the JetBrains extension to goose, replacing "YOUR_IDE_SPECIFIC_URL" in the instructions with the URL you copied:
       <Tabs groupId="interface">
         <TabItem value="ui" label="goose Desktop" default>
           1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
           2. Click `Extensions` on the sidebar
           3. Click `Add custom extension`
           4. On the `Add custom extension` modal, enter the following:
              - **Extension Name**: JetBrains
              - **Type**: Server-Sent Events (SSE)
              - **Endpoint**: YOUR_IDE_SPECIFIC_URL
           5. Click `Add Extension` to save the extension
           6. Navigate to the chat
         </TabItem>
         <TabItem value="cli" label="goose CLI">
           <CLIExtensionInstructions            
             name="jetbrains"
             description="Integrate goose with any JetBrains IDE"
             type="sse"
             url="YOUR_IDE_SPECIFIC_URL"
             timeout={300}
           />
         </TabItem>
       </Tabs>
  </TabItem>
  <TabItem value="earlier" label="2025.1 and earlier">

    Versions 2025.1 and earlier require installing the MCP Server plugin and using the [JetBrains MCP Proxy Server](https://github.com/JetBrains/mcp-jetbrains).

    :::tip TLDR
    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
      [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40jetbrains%2Fmcp-proxy&id=jetbrains&name=JetBrains&description=Integrate%20goose%20with%20any%20JetBrains%20IDE)
      </TabItem>
      <TabItem value="cli" label="goose CLI">
      **Command**
      ```sh
      npx -y @jetbrains/mcp-proxy
      ```
      </TabItem>
    </Tabs>

    **Required Setup**

    Add the [MCP Server plugin](https://plugins.jetbrains.com/plugin/26071-mcp-server) to your IDE.
    :::

    :::info
    Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
    :::

    <br/>
    Configure the extension using the MCP Server plugin and proxy server:

    1. Add the [MCP Server plugin](https://plugins.jetbrains.com/plugin/26071-mcp-server) to your IDE.

    2. Add the JetBrains extension to goose:

       <Tabs groupId="interface">
         <TabItem value="ui" label="goose Desktop" default>
           <GooseDesktopInstaller
             extensionId="jetbrains"
             extensionName="JetBrains"
             description="Integrate goose with any JetBrains IDE"
             command="npx"
             args={["-y", "@jetbrains/mcp-proxy"]}
             timeout={300}
           />
         </TabItem>
         <TabItem value="cli" label="goose CLI">
             <CLIExtensionInstructions
               name="jetbrains"
               description="Integrate goose with any JetBrains IDE"
               command="npx -y @jetbrains/mcp-proxy"
               timeout={300}
             />
         </TabItem>
       </Tabs>

  </TabItem>
</Tabs>

## Example Usage

In this example, I'm going to upgrade a Java project to the latest LTS version.

:::info LLM
Anthropic's Claude 4 Sonnet was used for this task.
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Open [IntelliJ](https://www.jetbrains.com/idea/download) (JetBrains' Java and Kotlin IDE)
   2. Open a new session in goose Desktop
   :::note
   You will interact with two separate apps: the goose Desktop app and the IntelliJ IDE.
   :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Open [IntelliJ](https://www.jetbrains.com/idea/download) (JetBrains' Java and Kotlin IDE)
  2. Open a terminal within your IDE and start a new goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

:::tip 
When using goose to refactor code, it's recommended to commit your code to version control _before_ making changes. This way, you have a clean snapshot to compare against, making it easy to review goose’s modifications, revert if needed, and track improvements.
:::

### goose Prompt
```
using the JetBrains IDE tools, upgrade this project to use the latest Java LTS version and refactor the codebase to use newer Java features where beneficial.
```

After 2 minutes, goose was done with the upgrade! 🎉

### goose Output

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





Using the JetBrains extension provided several advantages and made it easier for goose to:
- navigate through the codebase
- track changes
- make consistent modifications across files
- verify the content of files before and after changes

