---
sidebar_position: 6
title: Customizing Prompt Templates
sidebar_label: Prompt Templates
description: Learn how to customize the prompt templates that define gosling's behavior in different situations
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

gosling comes with built-in prompt templates that guide its behavior in different situations. You can edit these templates to customize how gosling responds, creates plans, decides what to save during compaction, and more.

## How It Works

gosling's default prompt templates are defined in the codebase and embedded in the application. You can override any default by creating a custom version in your local config directory (either directly or via gosling Desktop).

When you customize a template:

- Your customizations persist across gosling updates
- Changes to defaults in the codebase don't affect your customized templates
- You can reset to default templates at any time
- Changes take effect in new sessions

Your changes can range from major updates to minor adjustments such as:
- Edit `system.md` to have gosling respond in Dutch by adding an instruction to "Reply in Dutch"
- Edit `plan.md` to add time estimates by adding instructions to "Include an estimated time for each step (e.g., "~5 min", "~30 min", "~2 hours")."

See [Template Variable Syntax](#template-variable-syntax) for important information about modifying template variables.

:::info Related Configuration
Other gosling settings and features can also affect behavior or provide context, such as [config files](/docs/guides/config-files), [.goslinghints](/docs/guides/context-engineering/using-goslinghints), and [skills](/docs/guides/context-engineering/using-skills).
:::

## Managing Prompt Templates

<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
  
  gosling Desktop users can manage templates from the `Settings` page.

  **To customize a template:**

  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Click `Settings` in the sidebar
  3. Click the `Prompts` tab
  4. Click `Edit` next to the template you want to change
  5. Make your changes in the editor. You can click `Restore Default` to start over from the default template at any time.
  6. Click `Save` to apply your customization

  Customized prompt templates display a `Customized` badge.

  **To reset a template to its default:**

  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Click `Settings` in the sidebar
  3. Click the `Prompts` tab
  4. Click `Edit` next to the template you want to reset
  5. Click `Reset to Default` to delete your local template file

  Or click `Reset All` at the top of the tab to delete all of your local template files. 

  </TabItem>
  <TabItem value="cli" label="gosling CLI">

  gosling CLI users can edit template files directly in the file system.

  Custom templates are stored in:

  - **macOS/Linux:** `~/.config/gosling/prompts/`
  - **Windows:** `%APPDATA%\Block\gosling\config\prompts\`

  **To customize a template:**

  1. Create the `prompts` directory if it doesn't exist
  2. Copy the template file name from the table above (e.g., `system.md`)
  3. Create a file with that name in your prompts directory
  4. Add your custom content and save your changes. We recommend that you start by reviewing or copying the default template (linked in the [table](#available-prompt-templates) above).

  **To reset a template to its default:**

  1. Delete the template file from your `prompts` directory

  </TabItem>
</Tabs>

### Available Prompt Templates

The following default templates can be customized.

| Template | Description | Applies To |
|----------|-------------|------------|
| [system.md](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompts/system.md) | General system prompt defining gosling's role, capabilities, and response format | Desktop and CLI |
| [compaction.md](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompts/compaction.md) | Prompt for summarizing conversation history when context limits are reached | Desktop and CLI |
| [permission_judge.md](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompts/permission_judge.md) | Prompt for analyzing tool operations for read-only detection | Desktop and CLI |
| [plan.md](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompts/plan.md) | Instructions for creating detailed, actionable plans with clarifying questions | CLI only |
| [subagent_system.md](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompts/subagent_system.md) | System prompt for subagents spawned to handle specific tasks | Desktop and CLI |

Customizable templates are enumerated in the `TEMPLATE_REGISTRY` array in [`prompt_template.rs`](https://github.com/repo-makeover/gosling/blob/main/crates/gosling/src/prompt_template.rs).

### Template Variable Syntax

Templates use [Jinja2](https://jinja.palletsprojects.com/) syntax for dynamic content:

- `{{ variable }}` - Inserts a value (e.g., `{{ extensions }}` lists enabled extensions)
- `{% if condition %}...{% endif %}` - Conditional sections
- `{% for item in list %}...{% endfor %}` - Loops over items

Check out the default templates (linked to from the [table](#available-prompt-templates) above) to find common variables, such as `{{ extensions }}` and `{{ hints }}`.

#### Escaping Template Variables

If you need to include literal variable syntax in your templates without substitution, wrap it in single quotes:

```markdown
This will substitute: {{ variable }}
This will appear literally: {{'{{variable}}'}}
```

:::warning
Be careful when modifying template variables, as incorrect changes can break functionality. Test your changes in a new session to ensure they work as expected.
:::

