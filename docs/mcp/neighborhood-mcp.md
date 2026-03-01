<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/qY2i83l6dCQ" />

Order food from nearby restaurants — right from your goose chat. Sellers are currently US-based.

In this tutorial, you'll learn how to set up the Neighborhood extension and use it to discover nearby restaurants, browse menus, build a cart, and check out — all through natural conversation with goose.

## Quick Install

:::tip
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Install Neighborhood extension](goose://extension?type=streamable_http&url=https%3A%2F%2Fconnect.squareup.com%2Fv2%2Fmcp%2Fneighborhood&id=neighborhood&name=Neighborhood&description=Discover%20nearby%20restaurants%2C%20browse%20menus%2C%20and%20place%20takeout%20orders%20through%20natural%20conversation.)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streamable HTTP)` with:

  **Endpoint URL**

  ```
  https://connect.squareup.com/v2/mcp/neighborhood
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

:::info LLM
Anthropic's Claude 4 Opus was used for this example.
:::

### Step 1: Find restaurants near you

Ask goose to find lunch options nearby. You can include details like your location, dietary restrictions, and what you're up to — goose will factor it all in.

**Prompt:**
```
Use the neighborhood extension for this. I'm looking to get lunch today.
I'm at 375 W Broadway in New York. I'm playing tennis later tonight.
Could you suggest some options so I can play well?
```

<details>
<summary>goose Output</summary>

```
Let me find some restaurants near you!

Here are some restaurants near 375 W Broadway in SoHo! For pre-game fuel,
I'd recommend these options:

Best picks for a light, gluten-free meal:

Ba'al Falafel (71 Sullivan St) - Mediterranean food with falafel, hummus,
and fresh veggies — great carbs and protein

Kale & Things - Perfect for a healthy, nutrient-dense meal with greens and
lean proteins for sustained energy without feeling heavy

Tips for eating before playing sport:

- Eat 2-3 hours before playing to allow digestion
- Focus on complex carbs + lean protein (avoid heavy, fatty foods)
- Stay hydrated - consider grabbing a drink from Steam & Sip or Drip Drop Café

Would you like me to pull up the menu for any of these restaurants?
```

</details>

goose uses the Neighborhood extension's interactive UI to display restaurant cards right in the chat — each showing the name, category, and address with a **View menu** button.

[![Nearby restaurants displayed as interactive cards in goose](/img/neighborhood-restaurants-1.png)](/img/neighborhood-restaurants-1.png)

goose doesn't just list restaurants — it tailors its recommendations to your situation. Since you mentioned playing tennis later, goose suggests meals that are energizing but not too heavy, and even includes a pro tip on pre-match eating timing.

[![goose response with restaurant recommendations for pre-tennis meal](/img/neighborhood-restaurants-2.png)](/img/neighborhood-restaurants-2.png)

Click on any restaurant to view its menu.

### Step 2: Browse the menu

Ask goose to open a specific restaurant's menu. The interactive MCP app renders a full browsable menu directly in the chat — complete with category tabs, food photos, prices, and descriptions.

**Prompt:**
```
I'd like to view the Ba'al Falafel menu.
```

[![Ba'al Falafel menu displayed with category tabs, photos, prices, and descriptions in goose](/img/neighborhood-menu.png)](/img/neighborhood-menu.png)

Browse through all the available dishes — Sandwiches, Salads, Combo platters, Soups, Pastries, Sides, Drinks, and more. Ask goose to add anything that catches your eye to your cart.

### Step 3: Add to cart and check out

Tell goose what you'd like to order and it will add items to your cart. goose even takes your dietary preferences into account when suggesting items.

**Prompt:**
```
Add a Beets Apple Salad, Lentil Soup, and a Ginger Lemonade to my cart.
```

[![Order summary with items, subtotal, and checkout button](/img/neighborhood-cart.png)](/img/neighborhood-cart.png)

Your cart is ready! From here, just click the **Check out** button — it takes you straight to the payment page powered by Cash App. Complete your payment, and all that's left is to go pick up your lunch. 🎉

### More Prompt Ideas

Try combining Neighborhood with other goose extensions for even more useful workflows:

- **Pair with your calendar:** *"Check my calendar for today and find somewhere I can grab a quick pickup lunch before my 1pm meeting. I'm near Union Square."*
- **Track what you eat:** *"Order me a chicken bowl from that place on Sullivan St, and log the macros to my food diary."*
- **Plan for a group:** *"I'm meeting 3 friends near Bryant Park tonight. Find somewhere with good vegetarian options that's not too loud."*

## Video Walkthrough

Watch a full walkthrough of the Neighborhood extension in action:

<iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/DG1HUFsekyc"
  title="Neighborhood Extension"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
  allowFullScreen
></iframe>