On first use, goose asks for permission to collect anonymous usage data to help improve the product. You can change this setting at any time.

## Usage data collected

To respect your privacy, goose collects only anonymous usage metrics when you opt in. If enabled, the following data is collected:

- Operating system, version, and architecture
- goose version and install method
- Provider and model used
- Extensions and tool usage counts (names only)
- Session metrics (duration, interaction count, token usage)
- Error types (e.g., "rate_limit", "auth" - no details)

Collected usage data doesn't include your conversations, code, tool arguments, error messages, or any personal data.

:::info Provider Data Handling
Depending on the [LLMs](/docs/getting-started/providers) you use with goose, your conversations, prompts, and information accessed by goose might be sent to the provider and subject to their data retention and privacy policies.
:::

## Change Your Preference

To change your usage data collection preference:

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `Settings` in the sidebar
    3. Click the `App` tab
    4. In the `Privacy` section, toggle `Anonymous usage data` on or off
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Use the arrow keys to move through the options and press `Enter` to select. A solid dot shows your current selection.
    1. Run `goose configure`
    2. Choose `goose settings`
    3. Choose `Telemetry`
    4. Your current telemetry status is shown. Select `Yes` to enable anonymous usage data collection or `No` to disable it.
    
    ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  goose settings 
    │
    ◇  What setting would you like to configure?
    │  Telemetry 
    │
    ●  Current telemetry status: Disabled
    │  
    ◇  Share anonymous usage data to help improve goose?
    │  Yes 
    │
    └  Telemetry enabled - thank you for helping improve goose!
    └  Configuration saved successfully to /Users/julesv/.config/goose/config.yaml
    ```
  </TabItem>
</Tabs>

You can also set the `GOOSE_TELEMETRY_ENABLED` variable directly in your [`config.yaml` file](/docs/guides/config-files), or use it as an [environment variable](/docs/guides/environment-variables#security-and-privacy) to set telemetry status for a given session.