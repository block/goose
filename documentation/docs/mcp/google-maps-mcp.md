---
title: Google Maps Extension
description: Add Google Maps MCP Server as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

Server archived

This tutorial covers how to add the [Google Maps MCP Server](https://www.pulsemcp.com/servers/google-maps-docs) as a goose extension to enable geocoding, place searching, distance calculations, elevation data retrieval, and directions.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-google-maps&id=google-maps&name=Google%20Maps&description=Google%20Maps%20API%20integration&env=GOOGLE_MAPS_API_KEY%3DGoogle%20Maps%20API%20key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @modelcontextprotocol/server-google-maps
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  GOOGLE_MAPS_API_KEY: <YOUR_TOKEN>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface" defaultValue="ui">
  <TabItem value="ui" label="goose Desktop">
  <GooseDesktopInstaller
    extensionId="google-maps"
    extensionName="Google Maps"
    description="Google Maps API integration"
    command="npx"
    args={["-y", "@modelcontextprotocol/server-google-maps"]}
    envVars={[
      { name: "GOOGLE_MAPS_API_KEY", label: "Google Maps API Key" }
    ]}
    apiKeyLink="https://developers.google.com/maps/documentation/javascript/get-api-key"
    apiKeyLinkText="Google Maps API Key"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
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

  3. Give your extension a name
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
    │  Google Maps
    // highlight-end
    └ 
  ```

  4. Enter the command
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
    │  Google Maps
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-google-maps
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
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Google Maps
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-google-maps
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    └ 
  ``` 

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
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
    │  Google Maps
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-google-maps
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

  7. Obtain a [Google Maps API Key](https://developers.google.com/maps/documentation/javascript/get-api-key) and paste it in.

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
    │  Google Maps
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-google-maps
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
    │  GOOGLE_MAPS_API_KEY
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added github extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

goose acts as an autonomous agent that tracks a delivery driver’s location, updates the customer in real-time, and adjusts ETAs dynamically based on Google Maps traffic data.

goose can:

**Monitor Driver Location**
* goose gets the driver’s GPS coordinates every few seconds.
* If the driver’s location is significantly behind schedule, goose recalculates the ETA.

**Traffic-Aware ETA Adjustments**
* goose queries Google Maps for live traffic conditions.
* If delays are detected, goose updates the customer’s tracker with a new estimated time.

**Dynamic Customer Notifications**
* If the driver is stuck in traffic, goose sends an alert to the customer: “Your driver is running late due to traffic. Updated ETA: 7:25 PM.”
* If the driver is ahead of schedule, goose notifies the restaurant to prepare the food earlier.


### goose Prompt
```
Track the live GPS location of driver ID #12345. Query Google Maps for real-time traffic data and adjust the estimated delivery time if delays exceed 5 minutes. If ETA changes, update the customer’s live tracker and send an SMS notification. If the delay is greater than 20 minutes, check if another driver within a 1-mile radius can take over the delivery.
```

### goose Output

:::note CLI
Okay, I will track Driver #12345, query Google Maps for traffic data, adjust the ETA, and notify the customer.

**Track Driver's GPS Location**

*   **Driver ID:** #12345
*   **Starting Point:** 1955 Broadway, Suite 600, Oakland, CA 94612

Driver #12345's movement:

```
2025-02-23 08:20:00: Latitude: 37.8074, Longitude: -122.2697 (Starting point)
2025-02-23 08:21:00: Latitude: 37.8080, Longitude: -122.2690
2025-02-23 08:22:00: Latitude: 37.8090, Longitude: -122.2680
2025-02-23 08:23:00: Latitude: 37.8100, Longitude: -122.2670
```


**Geocode Destination Address**

*   **Customer Address:** 2111 Mission Street, San Francisco, CA

I will convert this address into geographic coordinates:


`─── maps_geocode | googlemaps ──────────────────────────`

address: 2111 Mission Street, San Francisco, CA

```
Latitude: 37.7631446
Longitude: -122.4192073
```


**Get Initial Directions and ETA**

I will get the initial route and ETA from the driver's starting point to the customer's address.


`─── maps_directions | googlemaps ──────────────────────────`

destination: 2111 Mission St, San Francisco, CA
mode: driving
origin: ...

```
Distance: 12.1 miles
Initial ETA: 21 minutes
Estimated arrival at: 08:41 AM
```


**Monitor Traffic and Adjust ETA**

I will monitor traffic updates and recalculate the ETA periodically.

```
Remaining Distance: 8 miles
Updated ETA: 15 minutes
Estimated arrival at: 08:45 AM
```

There is no significant delay.

*At 08:35 AM:* There's a traffic incident. 


`─── maps_directions | googlemaps ──────────────────────────`

destination: 2111 Mission St, San Francisco, CA
mode: driving
origin: Latitude: 37.8100, Longitude: -122.2670


I apologize, I am unable to use the lat/long for the request. I will call google maps api with names instead.

`─── maps_directions | googlemaps ──────────────────────────`

destination: 2111 Mission St, San Francisco, CA
mode: driving
origin: ...

```
Remaining Distance: 7 miles
Updated ETA: 26 minutes
Estimated arrival at: 08:46 AM
```


The ETA has increased by 5 minutes from the original ETA of 08:41 AM. The directions now include a message: "Expect delays due to congestion."

**Update Customer and Send SMS**

[... other extensions used (e.g. Webhooks, MySQL, ClickSend)...]

*   **Update Live Tracker:** The customer's live tracking interface is updated to reflect the new ETA of 08:46 AM.
*   **Send SMS Notification:** _"Your delivery is now expected to arrive at approximately 08:46 AM due to traffic. We apologize for the inconvenience."_
:::