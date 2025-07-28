---
title: mbot Extension
description: Control a MakeBlock mbot2 rover through MQTT and MCP as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/QKg2Q6YCzdw" />

This tutorial will get you started with [deemkeen's MQTT MCP server](https://github.com/deemkeen/mbotmcp) for the [MakeBlock mbot2 rover](https://www.makeblock.com/products/buy-mbot2), and outline some code changes we made along the way.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=/path/to/java&arg=-jar&arg=/path/to/mbotmcp-0.0.1-SNAPSHOT.jar&name=mbot2&description=mbot2&env=MQTT_SERVER_URI%3Dtcp://1.2.3.4:1883&env=MQTT_USERNAME%3Dyour_username&env=MQTT_PASSWORD%3Dyour_password)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  MQTT_SERVER_URI: tcp://1.2.3.4:1883
  MQTT_PASSWORD: <string or blank>
  MQTT_USERNAME: <string or blank>
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="mbot2"
    extensionName="mbot2"
    description="mbot2"
    command="/path/to/java"
    args={["-jar", "/path/to/mbotmcp-0.0.1-SNAPSHOT.jar"]}
    envVars={[
      { name: "MQTT_SERVER_URI", label: "tcp://1.2.3.4:1883" },
      { name: "MQTT_USERNAME", label: "your_username" },
      { name: "MQTT_PASSWORD", label: "your_password" }
    ]}
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  1. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension (SSE) 
    │  ○ Remote Extension (Streaming HTTP) 
    └ 
  ```

  2. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  mbot2
    // highlight-end
    └ 
  ```

  3. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  mbot2
    │
    // highlight-start
    ◆  What command should be run?
    │  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
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
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  mbot2
    │
    ◇  What command should be run?
    │  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ```  

  5. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  mbot2
    │
    ◇  What command should be run?
    │  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
    │
    ◆  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    │
    └ 
  ```  

  6. Add environment variables for MQTT
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  mbot2
    │
    ◇  What command should be run?
    │  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    ◇  Would you like to add a description?
    │  No
    │
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  MQTT_SERVER_URI
    │
    ◇  Environment variable value:
    │  tcp://1.2.3.4:1883
    │
    ◇  Add another environment variable?
    │  Yes
    │
    ◇  Environment variable name:
    │  MQTT_USERNAME
    │
    ◇  Environment variable value:
    │  username
    │
    ◇  Add another environment variable?
    │  Yes
    │
    ◇  Environment variable name:
    │  MQTT_PASSWORD
    │
    ◇  Environment variable value:
    │  password
    // highlight-end
    │
    └  Added mbot2 extension
  ```  

  </TabItem>
</Tabs>

    :::info 
    MQTT_USERNAME and MQTT_PASSWORD are required to exist, but can be empty strings if your MQTT server does not require authentication.
    :::

## Example Usage

The available commands allow you to drive the mbot2 rover around, including:

- "turn left" or "turn right""
- drive "forward" or "backward"
- "explore" randomly
- "stop" exploring
- "beep"

The default distance to travel is 70cm (about 27 inches), and the turn angles are set to 90 degrees. You can change these values in the [Python code on the mbot2](https://github.com/deemkeen/mbotmcp/blob/main/assets/mbot-mqtt.py). The mbot2 has a lot of other capabilities with the proximity sensors, lights, and color detection sensor on the bottom of the unit that you can add to the Python code, and will need to update [the Java code](https://github.com/deemkeen/mbotmcp/blob/main/src/main/java/de/emkeen/mbotmcp/service/BotService.java) to include those commands via MCP.

#### Goose Interaction

```
( O)> Let my mbot2 explore the area

    Okay, let's get the mbot2 rover moving. I will send it on a mission to explore the area.

    > Mbotexplore

( O)> stop the rover, turn left and move forward

    Alright, I will stop the rover, turn to the left, and then move it forward.

    > Mbotstop

    > Mbotleft

    > Mbotforward

( O)> let's move backward and beep so I know when it's finished

    Sure, I will move the rover backward and beep when it's done.

    > Mbotbackward

    > Mbotbeep

    Okay, the mbot2 has moved and beeped. What else would you like to do with the rover?

```
