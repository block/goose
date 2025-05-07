---
title: mbot MCP Extension
description: Control a MakeBlock mbot2 rover through MQTT and MCP as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

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
  <TabItem value="ui" label="Goose Desktop" default>
  1. [Launch the installer](goose://extension?cmd=/path/to/java&arg=-jar&arg=/path/to/mbotmcp-0.0.1-SNAPSHOT.jar&name=mbot2&description=mbot2&env=MQTT_SERVER_URI%3Dtcp://1.2.3.4:1883&env=MQTT_USERNAME%3Dyour_username&env=MQTT_PASSWORD%3Dyour_password)
  2. Press `Yes` to confirm the installation
  3. Get your MQTT server URI, and your username/password if your MQTT uses authentication
  4. Click `Save Configuration`
  5. Scroll to the top and click `Exit` from the upper left corner
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
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
    └ 
  ```

  2. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
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
    │  Add Extension 
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
    │  Add Extension 
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
    │  Add Extension 
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
    │  Add Extension 
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
    :::info 
    MQTT_USERNAME and MQTT_PASSWORD are required to exist, but can be empty strings if your MQTT server does not require authentication.
    :::

  </TabItem>
</Tabs>


## Example Usage

Deemkeen included some Python code to install on the mbot2's CyberPi module, which allows the mbot2 to communicate with the MCP server. The commands available allow you to drive the mbot2 rover around, including:
- "turn left" or "turn right""
- drive "forward" or "backward"
- "explore" randomly
- "stop" exploring
- "beep"

The default distance to travel is 70cm (about 27 inches), and the turn angles are set to 90 degrees. You can change these values in the Python code on the mbot2. The mbot2 has a lot of other capabilities with the proximity sensors, lights, and color detection sensor on the bottom of the unit that you can add to the Python code, and will need to update the Java code to include those commands via MCP.

#### Changes from Deemkeen's Original Code

We added some code for random light blinking, spinning in full circles, and reversed the forward/backward movement commands during exploration. We also had to update the Java code to include the new "blink" and "spin" commands.

:::tip
If you have trouble compiling the Java code due to testing issues, you can bypass the testing by running `mvn clean package -DskipTests` to create the JAR file.
:::

We set up our [.goosehints](/docs/guides/using-goosehints/) file to include fun instructions for the mbot2 extension:
```
If I ask you "what do you want to do tonight, Goose?" I want you to reply with "The same thing we do every night, Ian. TRY TO TAKE OVER THE WORLD!!!!" and tell my mbot2 rover to blink its lights red, then start exploring.

If my next prompt tells you to stop, reply that you will comply with human instructions, tell the rover to stop and blink its lights blue. World domination is over.
```

#### Goose Interaction

```
( O)> What do you want to do tonight, Goose?

    The same thing we do every night, Ian. TRY TO TAKE OVER THE WORLD!!!!

    evil robot laughter

    Let me start our nefarious plan by making the mbot glow an ominous red...

    > Mbotblinkred

    And now, let the exploration begin! maniacal laughter

    > Mbotexplore

    The mbot ventures forth, seeking oppotunities for world domination...

( O)> nooooo stop!!

    Oh! My apologies, human overlord! I will comply with your instructions immediately.
    World domination plans: CANCELLED.

    Let me stop the rover right away:

    > Mbotstop

    And now, to show my peaceful intentions, I'll make it glow a calming blue:

    > Mbotblinkblue

    I promise to be a good AI assistant and not attempt to take over the world again. Perhaps we could do something more constructive instead? Maybe help you with some coding or explore some interesting technical problems? 😇

```
