This tutorial covers how to add the [prompts.chat MCP Server](https://prompts.chat) as a goose extension to enable access to thousands of AI prompts directly in your AI assistant.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=@fkadev/prompts.chat-mcp@latest&id=prompts-chat-mcp&name=prompts.chat&description=Access%20thousands%20of%20AI%20prompts%20directly%20in%20your%20AI%20assistant&env=PROMPTS_API_KEY%3DAPI%20Key%20to%20save%20and%20list%20private%20prompts)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @fkadev/prompts.chat-mcp@latest
  ```
  </TabItem>
</Tabs>
  **Environment Variable (Optional)**
  ```
  PROMPTS_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="prompts-chat-mcp"
    extensionName="prompts.chat"
    description="Access thousands of AI prompts directly in your AI assistant"
    command="npx"
    args={["-y", "@fkadev/prompts.chat-mcp@latest"]}
    envVars={[
      { name: "PROMPTS_API_KEY", label: "API Key to save and list private prompts on prompts.chat (optional)" }
    ]}
    apiKeyLink="https://prompts.chat"
    apiKeyLinkText="prompts.chat API Key"
  />
</TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="prompts.chat"
      description="Access thousands of AI prompts directly in your AI assistant"
      command="npx -y @fkadev/prompts.chat-mcp@latest"
      envVars={[
        { key: "PROMPTS_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          The PROMPTS_API_KEY is optional. If you want to save and list private prompts, obtain your API key from{" "}
          <a href="https://prompts.chat" target="_blank" rel="noopener noreferrer">
            prompts.chat
          </a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

The prompts.chat extension provides access to a curated library of AI prompts that you can search and use directly within goose. This is useful when you need inspiration or want to leverage proven prompt patterns for specific tasks.

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

### Searching for Prompts

#### goose Prompt

```
Search for prompts about code review
```

#### goose Output

```
I found several prompts related to code review:

1. **Code Review Assistant** - A comprehensive prompt for reviewing code quality, 
   security, and best practices
2. **Pull Request Reviewer** - Helps analyze pull requests and suggest improvements
3. **Security Code Audit** - Focuses on identifying security vulnerabilities

Would you like me to retrieve any of these prompts?
```

### Using a Prompt

#### goose Prompt

```
Get the Code Review Assistant prompt and use it to review my current file
```

#### goose Output

```
I've retrieved the Code Review Assistant prompt. Let me apply it to analyze your code...

[Applying prompt guidelines to review your code]

Here's my review based on the prompt's framework:

✅ Code structure and organization
✅ Naming conventions
⚠️ Consider adding error handling for edge cases
⚠️ Documentation could be improved for public methods

Would you like me to help address any of these findings?
```