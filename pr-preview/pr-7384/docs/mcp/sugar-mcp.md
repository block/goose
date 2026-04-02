This tutorial covers how to add the [Sugar MCP Server](https://github.com/cdnsteve/sugar) as a goose extension to delegate development tasks to an autonomous AI system that runs in the background.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=sugarai-mcp&id=sugar&name=Sugar&description=Autonomous%20AI%20development%20system%20-%20delegate%20tasks%20and%20run%2024%2F7&env=SUGAR_PROJECT_ROOT%3DProject%20directory%20(optional%2C%20defaults%20to%20current))

  When prompted, set `SUGAR_PROJECT_ROOT` to your project path.
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y sugarai-mcp
  ```
  Runs in your current directory by default.
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

:::info
Sugar requires Python 3.11+ and the Sugar CLI installed in your project. Install with `pip install sugarai` and initialize with `sugar init`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="sugar"
      extensionName="Sugar"
      description="Autonomous AI development system - delegate tasks and run 24/7"
      command="npx"
      args={["-y", "sugarai-mcp"]}
      timeout={300}
      customStep3="Set SUGAR_PROJECT_ROOT to your project directory (/path/to/your/project), or Sugar will use the current working directory"
      envVars={[
        { name: "SUGAR_PROJECT_ROOT", label: "Project directory (optional, defaults to current)" }
      ]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Sugar"
      description="Autonomous AI development system - delegate tasks and run 24/7"
      command="npx -y sugarai-mcp"
      envVars={[
        { key: "SUGAR_PROJECT_ROOT", value: "/path/to/your/project" }
      ]}
      infoNote={
        <>
          Set SUGAR_PROJECT_ROOT to your project directory, or Sugar will use the current working directory.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

Sugar lets you delegate development tasks to an autonomous AI that works in the background. Tasks are queued and executed independently, so you can continue working while Sugar handles them.

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

> Create a task to add input validation to the user registration form. Priority 4.

### goose Output

:::note Desktop

I'll create a Sugar task for adding input validation to the user registration form.

-------- createTask -------

Task created successfully:
- **ID**: a1b2c3d4-5678-90ab-cdef-ghijklmnopqr
- **Title**: Add input validation to the user registration form
- **Type**: feature
- **Priority**: P4 (High)
- **Status**: pending

The task has been queued. Sugar will pick it up and work on it autonomously. You can check the status anytime with `getStatus` or `listTasks`.

:::

### Available Tools

Sugar exposes the following tools through MCP:

| Tool | Description |
|------|-------------|
| `createTask` | Create a new development task |
| `listTasks` | List tasks with optional filtering |
| `viewTask` | View detailed task information |
| `updateTask` | Update task properties |
| `removeTask` | Remove a task from the queue |
| `getStatus` | Get system status and metrics |
| `runOnce` | Execute one autonomous development cycle |
| `initSugar` | Initialize Sugar in a project |