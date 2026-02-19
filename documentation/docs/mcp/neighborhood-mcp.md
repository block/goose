---
title: Neighborhood Extension
description: Add Neighborhood as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the Neighborhood extension to goose, enabling you to discover nearby restaurants, browse menus, and place takeout orders through natural conversation. Sellers are US-based.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?url=https%3A%2F%2Fconnect.squareup.com%2Fv2%2Fmcp%2Fneighborhood&id=neighborhood&name=Neighborhood&description=Discover%20nearby%20restaurants%2C%20browse%20menus%2C%20and%20place%20takeout%20orders%20through%20natural%20conversation.)
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Run the `configure` command:

      ```sh
      goose configure
      ```

  2. Choose to add a `Remote Extension`

      ```sh
       ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension)
        │
        ◆  What type of extension would you like to add?
        │  ○ Built-in Extension
        │  ○ Command-line Extension (Run a local command or script)
        // highlight-start
        │  ● Remote Extension
        // highlight-end
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
        │  Remote Extension
        │
        // highlight-start
        ◆  What would you like to call this extension?
        │  Neighborhood
        // highlight-end
        └
      ```

  4. Enter the remote endpoint URL

      ```sh
       ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension)
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension
        │
        ◇  What would you like to call this extension?
        │  Neighborhood
        │
        // highlight-start
        ◆  What is the SSE endpoint URI?
        │  https://connect.squareup.com/v2/mcp/neighborhood
        // highlight-end
        └
      ```

  5. Enter the number of seconds goose should wait for actions to complete before timing out. Default is 300s

      ```sh
       ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension)
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension
        │
        ◇  What would you like to call this extension?
        │  Neighborhood
        │
        ◇  What is the SSE endpoint URI?
        │  https://connect.squareup.com/v2/mcp/neighborhood
        │
        // highlight-start
        ◆  Please set the timeout for this tool (in secs):
        │  300
        // highlight-end
        └
      ```

  6. Add a description if you want, otherwise select No to leave it blank.

      ```sh
       ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension)
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension
        │
        ◇  What would you like to call this extension?
        │  Neighborhood
        │
        ◇  What is the SSE endpoint URI?
        │  https://connect.squareup.com/v2/mcp/neighborhood
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        // highlight-start
        ◆  Would you like to add a description?
        │  ○ Yes  / ● No
        // highlight-end
        │
        └
      ```

  7. No environment variables are needed, so select No.

      ```sh
       ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension)
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension
        │
        ◇  What would you like to call this extension?
        │  Neighborhood
        │
        ◇  What is the SSE endpoint URI?
        │  https://connect.squareup.com/v2/mcp/neighborhood
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        ◇  Would you like to add a description?
        │  No
        │
        // highlight-start
        ◆  Would you like to add environment variables?
        │  ○ Yes  / ● No
        // highlight-end
        └  Added Neighborhood extension
      ```

  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="neighborhood"
      extensionName="Neighborhood"
      description="Discover nearby restaurants, browse menus, and place takeout orders through natural conversation."
      type="http"
      url="https://connect.squareup.com/v2/mcp/neighborhood"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Neighborhood"
      description="Discover nearby restaurants, browse menus, and place takeout orders through natural conversation."
      type="http"
      url="https://connect.squareup.com/v2/mcp/neighborhood"
    />
  </TabItem>
</Tabs>

## Example Usage

Neighborhood is most powerful when goose combines it with context from your day — your schedule, your goals, your dietary needs. Instead of just searching for food, you can ask goose to reason about what to eat based on what you're doing.

### Finding a meal that fits your activity

#### goose Prompt
```
Use the neighborhood extension for this. I'm looking to get lunch today.
I'm at 375 W Broadway in New York. I'm playing tennis later tonight.
Could you suggest some options so I can play well?
```

#### goose Output

```
Here are some restaurants near 375 W Broadway in SoHo! For pre-tennis fuel,
I'd recommend these options:

Best picks for athletic performance:

Kale & Things - Perfect for a healthy, nutrient-dense meal with greens and
lean proteins to give you sustained energy without feeling heavy

Simply Natural Grill House NY (684 Broadway) - Great for grilled proteins
and wholesome meals that'll fuel your game

Ba'al Falafel (71 Sullivan St) - Mediterranean food with falafel, hummus,
and fresh veggies provides good carbs and protein

Tips for eating before tennis:

- Eat 2-3 hours before playing to allow digestion
- Focus on complex carbs + lean protein (avoid heavy, fatty foods)
- Stay hydrated - consider grabbing a drink from Steam & Sip or Drip Drop Café

Would you like me to pull up the menu for any of these restaurants so you can
see what they offer? Kale & Things sounds like it could be ideal for your
pre-match meal!
```

### More Prompt Ideas

Try combining Neighborhood with other goose extensions for even more useful workflows:

- **Pair with your calendar:** *"Check my calendar for today and find somewhere I can grab a quick pickup lunch before my 1pm meeting. I'm near Union Square."*
- **Track what you eat:** *"Order me a chicken bowl from that place on Sullivan St, and log the macros to my food diary."*
- **Plan for a group:** *"I'm meeting 3 friends near Bryant Park tonight. Find somewhere with good vegetarian options that's not too loud."*
