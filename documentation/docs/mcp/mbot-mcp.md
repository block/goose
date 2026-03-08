---
title: mbot Extension
description: Control a MakeBlock mbot2 rover through MQTT and MCP as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/QKg2Q6YCzdw" />

This tutorial will get you started with [deemkeen's MQTT MCP server](https://github.com/deemkeen/mbotmcp) for the [MakeBlock mbot2 rover](https://www.makeblock.com/products/buy-mbot2), and outline some code changes we made along the way.

:::tip TLDR
  **Command**
  ```sh
  /path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar
  ```

  **Environment Variables**
  ```
  MQTT_SERVER_URI: tcp://1.2.3.4:1883
  MQTT_PASSWORD: <string or blank>
  MQTT_USERNAME: <string or blank>
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="mbot2"
      description="Control a MakeBlock mbot2 rover through MQTT and MCP"
      command="/path/to/java -jar /path/to/mbotmcp-0.0.1-SNAPSHOT.jar"
      envVars={[
        { key: "MQTT_SERVER_URI", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" },
        { key: "MQTT_USERNAME", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" },
        { key: "MQTT_PASSWORD", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      commandNote={
        <>
          Replace <code>/path/to/java</code> and <code>/path/to/mbotmcp-0.0.1-SNAPSHOT.jar</code> with your actual Java installation and JAR file paths.
        </>
      }
      infoNote={
        <>
          MQTT_SERVER_URI has a value like <code>tcp://1.2.3.4:1883</code>. MQTT_USERNAME and MQTT_PASSWORD are required to exist, but can be empty strings if your MQTT server does not require authentication.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

The available commands allow you to drive the mbot2 rover around, including:

- "turn left" or "turn right""
- drive "forward" or "backward"
- "explore" randomly
- "stop" exploring
- "beep"

The default distance to travel is 70cm (about 27 inches), and the turn angles are set to 90 degrees. You can change these values in the [Python code on the mbot2](https://github.com/deemkeen/mbotmcp/blob/main/assets/mbot-mqtt.py). The mbot2 has a lot of other capabilities with the proximity sensors, lights, and color detection sensor on the bottom of the unit that you can add to the Python code, and will need to update [the Java code](https://github.com/deemkeen/mbotmcp/blob/main/src/main/java/de/emkeen/mbotmcp/service/BotService.java) to include those commands via MCP.

#### goose Interaction

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
