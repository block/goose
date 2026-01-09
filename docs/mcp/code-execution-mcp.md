The Code Execution extension enables [Code Mode](/docs/guides/managing-tools/code-mode), a programmatic approach for interacting with MCP tools.

In Code Mode, the LLM discovers which tools are available from your enabled extensions and writes JavaScript code that goose runs in one execution instead of calling tools directly and one at a time. This helps manage context window usage more efficiently when multiple extensions are enabled and when performing workflows with multiple tool calls.

## Configuration

<PlatformExtensionNote defaultEnabled={false} />

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Code Execution"
    description="Execute JavaScript code to interact with MCP tools efficiently"
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
  │  ● code_execution
  // highlight-end  
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Example Usage

In this example, we'll ask goose to compile a report that requires multiple tool calls.

### goose Prompt

```
Create a LOG.md file with the current git branch, last 3 commits, and the version from package.json
```

### goose Output

:::note Desktop
I'll help you create a LOG.md file with the git branch, last 3 commits, and version from package.json. Let me gather all this information in one operation.

`Execute Code code: import { shell, text_editor } from "developer" ...`

Let me check the package.json path first:

`Execute Code code: import { shell, text_editor } from "developer" ...`

Perfect! I've created the **LOG.md** file with:

- ✅ **Current git branch**
- ✅ **Last 3 commits** (with hash, message, author, and relative time)
- ✅ **Version** from ui/desktop/package.json

The file has been saved to the root directory as `LOG.md`.
:::