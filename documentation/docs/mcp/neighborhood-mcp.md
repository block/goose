---
title: Neighborhood Extension
description: Discover nearby restaurants and place takeout orders with goose
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The Neighborhood extension allows you to discover nearby restaurants, browse menus, and place takeout orders through natural conversation with goose. The extension connects to Square's Neighborhood service, which features US-based sellers.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?url=https%3A%2F%2Fconnect.squareup.com%2Fv2%2Fmcp%2Fneighborhood&id=neighborhood&name=Neighborhood&description=Discover%20nearby%20restaurants%2C%20browse%20menus%2C%20and%20place%20takeout%20orders%20through%20natural%20conversation.)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  goose session --with-remote-extension "https://connect.squareup.com/v2/mcp/neighborhood"
  ```
  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  1. [Launch the installer](goose://extension?url=https%3A%2F%2Fconnect.squareup.com%2Fv2%2Fmcp%2Fneighborhood&id=neighborhood&name=Neighborhood&description=Discover%20nearby%20restaurants%2C%20browse%20menus%2C%20and%20place%20takeout%20orders%20through%20natural%20conversation.)
  2. Press `Yes` to confirm the installation
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

## Example Usage

Use the Neighborhood extension to find restaurants near you and place orders through natural conversation.

### Finding Nearby Restaurants

#### goose prompt

```
Find pizza places near me that are open right now
```

#### goose uutput

```
I found several pizza places near your location that are currently open:

1. **Joe's Pizza** - 0.3 miles away
   - Rating: 4.5 stars
   - Known for: New York style slices
   - Open until 11:00 PM

2. **Napoli's Pizzeria** - 0.7 miles away
   - Rating: 4.3 stars
   - Known for: Wood-fired Neapolitan pizza
   - Open until 10:00 PM

3. **Pizza Palace** - 1.2 miles away
   - Rating: 4.1 stars
   - Known for: Deep dish and specialty pizzas
   - Open until 9:30 PM

Would you like me to show you the menu for any of these restaurants?
```

### Browsing a menu

#### goose Prompt

```
Show me the menu for Joe's Pizza
```

#### goose output

```
Here's the menu for Joe's Pizza:

**Pizzas (available by slice or whole pie)**
- Cheese Pizza - $3.50/slice, $22/pie
- Pepperoni - $4.00/slice, $26/pie
- Margherita - $4.50/slice, $28/pie
- Meat Lovers - $5.00/slice, $32/pie
- Veggie Supreme - $4.50/slice, $28/pie

**Sides**
- Garlic Knots (6) - $5.00
- Caesar Salad - $8.00
- Buffalo Wings (10) - $12.00

**Drinks**
- Soda (can) - $2.00
- Bottled Water - $1.50

Would you like to place an order?
```

### Placing an Order

#### goose prompt

```
Order a large pepperoni pizza and garlic knots from Joe's Pizza for pickup
```

#### goose output

```
I've prepared your order from Joe's Pizza:

**Order Summary:**
- 1x Pepperoni Pizza (whole pie) - $26.00
- 1x Garlic Knots (6) - $5.00

**Subtotal:** $31.00
**Tax:** $2.79
**Total:** $33.79

**Pickup Time:** Ready in approximately 20-25 minutes

Would you like me to confirm this order? I'll need your payment information to complete the purchase.
```

:::info
The Neighborhood extension currently supports US-based sellers only. Restaurant availability depends on your location and the time of day.
:::
