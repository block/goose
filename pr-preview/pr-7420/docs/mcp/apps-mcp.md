The Apps extension lets you create, manage, and launch simple custom apps through chat. This feature is ideal for:
- Quick utilities and tools like calculators and converters
- Data visualizations like graphs and dashboards
- Simple games and interactive widgets

Custom apps are single-file HTML applications with JavaScript, CSS, and HTML (no external dependencies or npm packages). They're exposed as [MCP App resources](/docs/guides/interactive-chat/mcp-ui) that run in standalone, sandboxed windows that you can launch from the `Apps` page or by asking goose in chat.

You can create, modify, and delete apps through chat without needing to edit files directly. Behind the scenes, the Apps extension stores each app as an HTML file in:

- **macOS/Linux:** `~/.local/share/goose/apps/`
- **Windows:** `%APPDATA%\Block\goose\data\apps\`

## Configuration

<PlatformExtensionNote/>

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Apps"
    description="Create and manage custom goose apps through chat. Apps are HTML/CSS/JavaScript and run in sandboxed windows."
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
  │  ● apps
  // highlight-end  
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Example Usage

Let's ask goose to create a utility app.

:::info LLM
Anthropic's Claude Sonnet 4.5 was used for this example.
:::

### goose Prompt

> _Create a JSON formatter app_

### goose Output

:::note Desktop
I'll create a JSON formatter app for you. This will be a useful tool for formatting, validating, and beautifying JSON data.

Perfect! I've created a JSON formatter app called "json-beautifier" that should have opened in a new window. 

The app includes:

✅ **Input & Output Areas**: Split view with input on the left and formatted output on the right  
✅ **Format Button**: Prettifies JSON with proper indentation  
✅ **Minify Button**: Compresses JSON to a single line  
✅ **Validate Button**: Checks JSON validity and shows errors  
✅ **Copy Button**: Copies the formatted JSON to clipboard  
✅ **Clear Button**: Resets everything  
✅ **Syntax Highlighting**: Color-coded keys, strings, numbers, and booleans  
✅ **Indentation Options**: Choose between 2 or 4 spaces  
✅ **Error Display**: Clear error messages in red  
✅ **Character/Line Count**: Shows statistics about your JSON  
✅ **Dark Theme**: Modern, easy-on-the-eyes design

You can paste JSON into the left panel and use the buttons to format, minify, validate, or copy it. The app should be ready to use right away!
:::

### Results

Here's what the JSON formatter looks like:

[![JSON Formatter App](/img/apps-extension-results.png)](/img/apps-extension-results.png)

The first iteration of my app looks and works great, but you can just ask goose to add features, change the styling, and more!