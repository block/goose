---
sidebar_position: 2
title: Adjusting Tool Output Verbosity
sidebar_label: Adjust Tool Output
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
Response Styles customize how tool interactions are displayed in the goose Desktop chat window. 

To change this setting:
1. Click the <PanelLeft className="inline" size={16} /> button on the top-left to open the sidebar.
2. Click the `Settings` button on the sidebar.
3. Click `Chat`.
4. Under `Response Styles`, select either `Detailed` or `Concise`.

- **Concise** (Default)
    - Tool calls are collapsed by default
    - Shows only which tool goose used
    - Best for users focusing on results rather than technical details

- **Detailed**
    - Tool calls are expanded by default
    - Shows the details of tool calls and their responses
    - Best for debugging or learning how goose works

This setting only affects the default state of tool calls in the conversation. You can always manually expand or collapse any tool call regardless of your chosen style.

</TabItem>
  <TabItem value="cli" label="goose CLI">
When working with the goose CLI, you can control the verbosity of tool output.

To adjust the tool output, run:

```sh
goose configure
```

Then choose `Adjust Tool Output`

```sh
┌   goose-configure 
│
◆  What would you like to configure?
│  ○ Configure Providers 
│  ○ Add Extension 
│  ○ Toggle Extensions 
│  ○ Remove Extension
// highlight-next-line
│  ● Adjust Tool Output (Show more or less tool output)
└  
```

Next, choose one of the available modes:

```sh
┌   goose-configure 
│
◇  What would you like to configure?
│  Adjust Tool Output 
│
// highlight-start
◆  Which tool output would you like to show?
│  ○ High Importance 
│  ○ Medium Importance 
│  ○ All 
// highlight-end
└  
```

- **High Importance**
    - Shows only the most important tool outputs
    - Most minimal output level

- **Medium Importance**
    - Shows medium and high importance outputs
    - Example: Results of file-write operations

- **All**
    - Shows all tool outputs
    - Example: Shell command outputs
    - Most verbose level
 </TabItem>
</Tabs>