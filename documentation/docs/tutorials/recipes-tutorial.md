---
description: Learn how to create and use goose recipes with this comprehensive tutorial covering prompts, parameters, and MCP servers
---

# goose Recipes

goose recipes are files that contain all the details to allow goose to do one specific task. Since they are contained in just one file, they are easy to share through all the normal ways we share files, including version management systems like git. Let's get started with the simplest recipe possible.

## The Simplest Recipe

The simplest recipe is basically just a prompt. This might seem not all that useful—after all I can just share my prompt on Slack or email—but it turns out that the most important reason users can't get agents to do what they want is that their prompts are too short and that they don't iterate enough on those prompts. Keeping prompts in a text file helps with both these things. 

Here's a recipe that will plan a trip to Europe:

```yaml
title: Trip planner
description: Plan your next trip
prompt: |
 Help the user plan a trip to Europe for 14 days.
 Create a detailed itinerary that includes:
  - places to visit
  - activities to do
  - local cuisine to try
  - a rough budget estimate
```

You can run it from the command line using:

```sh
goose run --recipe trip.yaml
```

## Extensions

goose recipes have a section where you can specify which [extensions](/docs/guides/recipes/recipe-reference#extensions) goose can use during execution. goose will only use the ones you specify. 

Let's say we want to make sure we have good weather during our Europe trip. We can just add a weather extension (this example uses the [weather-mcp-server](https://github.com/TuanKiri/weather-mcp-server) by TuanKiri under the MIT License) to our recipe, modify the prompt a bit and now goose will check the weather before adding a city to our trip.

```yaml
title: Trip planner
description: Plan your next trip
prompt: |
 Help the user plan a trip to Europe for 14 days. Create a detailed itinerary that includes:
  - places to visit
  - activities to do
  - local cuisine to try
  - a rough budget estimate
 Ensure that the user has good weather throughout their trip. Optimize their trip based on the forecast in potential locations.
extensions:
  - type: stdio
    name: weathermcpserver
    cmd: /Users/svega/Development/weather-mcp-server/weather-mcp-server
    args: []
    timeout: 300
    description: "Weather data for trip planning"
    env_keys:
      - WEATHER_API_KEY
```

## Parameters

We can make our recipes dynamic by adding parameters. Parameters are variables that are provided by the user of our recipes. They each have a data type and a requirement field that defines if they are required, optional or provided by the user. We can generalize our trip recipe by adding a parameter for the destination and the length of the trip:

```yaml
parameters:
  - key: destination
    input_type: string
    requirement: required
    description: Destination for the trip. Should be a large region with multiple climates.
  - key: duration
    input_type: number
    requirement: required
    description: Number of days for the trip.
```

Recipes use a template system that lets you insert variables like `{{ destination }}` which get filled in with the actual values you provide. Once you've updated the prompt with the right details, you can run your new recipe like this to get a plan for a 14 day trip to Africa:

```sh
goose run --recipe trip.yaml --params destination=Africa --params duration=14
```


## Settings

By default, goose uses the `temperature` and `model` you've already chosen, which usually works just fine. But sometimes you might want more control. For example, when performing a subjective task like planning a trip, it can help to turn up the `temperature` setting. Think of temperature like a creativity dial - the higher it is, the more varied and unexpected the results. If the first suggestion isn't quite right, the user can just run the recipe again to get a new one.

You can also specify which AI provider and model to use for a specific recipe:

```yaml
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.8
```

The available settings are:
- `goose_provider`: The AI provider (e.g., "anthropic", "openai")
- `goose_model`: The specific model name
- `temperature`: Controls creativity/randomness (0.0-1.0, higher = more creative)

These settings will override your default goose configuration when this recipe runs.

## External Files

Sometimes, you'll want to give the agent access to extra information without cramming all that data into the prompt. Instead of pasting everything in, you can keep the data in a separate file and point the recipe to it.

To help with this, recipes include a built-in variable called `{{ recipe_dir }}`, which lets you reference files stored alongside your recipe. For example, you could download the UNESCO list from [Kaggle](https://www.kaggle.com/datasets/ramjasmaurya/unesco-heritage-sites2021?resource=download) and use it in your travel planning recipe.

Then we reference the file in our prompt like:

```yaml
prompt: |
 You can use the \{\{ recipe_dir \}\}/unesco.csv file to 
 check information on UNESCO world heritage sites to
 include in your travel plan.
```

We also need to specify an extension to read files:

```yaml
extensions:
 - type: builtin
   name: developer
   display_name: Developer
   timeout: 300
   bundled: true
```

Here we add the [Developer extension](/docs/mcp/developer-mcp) which provides the ability to read files for relevant information.

:::info Example Recipe Output

<details>
<summary>View detailed 10-day European itinerary</summary>

Based on the UNESCO World Heritage site information and the current weather forecasts, here's a detailed 10-day European itinerary:

# 10-Day European Adventure Itinerary

This itinerary takes you through three of Europe's most beautiful and culturally rich countries: France, Italy, and the Czech Republic. You'll experience world-class museums, UNESCO World Heritage sites, delicious cuisine, and vibrant local culture.

#### Day 1-3: Paris, France 🇫🇷

**Day 1: Arrival in Paris**
- **Morning**: Arrive at Charles de Gaulle Airport, transfer to hotel
- **Afternoon**: Leisurely walk along the Seine River, visit Notre-Dame Cathedral (exterior view due to reconstruction)
- **Evening**: Dinner in the Latin Quarter (Budget: €30-40)
  - Try classic French onion soup and coq au vin

**Weather forecast**: Pleasant temperatures around 27°C (81°F), partly cloudy

**Day 2: Paris Highlights**
- **Morning**: Visit the Louvre Museum (Budget: €17)
- **Afternoon**: Explore Tuileries Garden and Champs-Élysées
- **Evening**: Eiffel Tower visit for sunset views (Budget: €26.80 for summit access)
  - Dinner near Trocadéro (Budget: €35-45)
  - Try escargot and beef bourguignon

**Weather forecast**: Warm at 31°C (88°F), clear skies

**Day 3: Versailles Day Trip**
- **Morning**: Day trip to Palace of Versailles, UNESCO World Heritage Site (Budget: €21 for palace access)
- **Afternoon**: Explore the magnificent gardens
- **Evening**: Return to Paris, dinner in Montmartre (Budget: €30-40)
  - Try crêpes and duck confit

**Weather forecast**: Warm at 30°C (86°F), slight chance of rain

#### Day 4-6: Rome, Italy 🇮🇹

**Day 4: Travel to Rome**
- **Morning**: Flight from Paris to Rome (Budget: €100-150)
- **Afternoon**: Check in to hotel, explore the Spanish Steps and Trevi Fountain
- **Evening**: Dinner in Trastevere neighborhood (Budget: €25-35)
  - Try authentic cacio e pepe and carbonara pasta

**Weather forecast**: Hot at 35°C (95°F), clear skies

**Day 5: Ancient Rome**
- **Morning**: Visit the Colosseum and Roman Forum (Budget: €16 combined ticket)
- **Afternoon**: Palatine Hill and Circus Maximus
- **Evening**: Dinner near Campo de' Fiori (Budget: €30-40)
  - Try Roman-style pizza and saltimbocca alla romana

**Weather forecast**: Hot at 35°C (95°F), mostly sunny

**Day 6: Vatican City**
- **Morning**: Vatican Museums and Sistine Chapel (Budget: €17)
- **Afternoon**: St. Peter's Basilica and Square (UNESCO World Heritage Site)
- **Evening**: Dinner in Prati district (Budget: €30-40)
  - Try suppli (rice balls) and osso buco

**Weather forecast**: Hot at 34°C (93°F), partly cloudy

#### Day 7-10: Prague, Czech Republic 🇨🇿

**Day 7: Travel to Prague**
- **Morning**: Flight from Rome to Prague (Budget: €100-150)
- **Afternoon**: Check in to hotel, explore Old Town Square
- **Evening**: Dinner in Old Town (Budget: €20-30)
  - Try traditional goulash and svíčková (marinated beef)

**Weather forecast**: Pleasant at 29°C (84°F), partly cloudy

**Day 8: Prague Castle and Lesser Town**
- **Morning**: Visit Prague Castle complex (UNESCO World Heritage Site) (Budget: 250 CZK/€10)
- **Afternoon**: Explore Lesser Town and Charles Bridge
- **Evening**: Dinner with views of the Vltava River (Budget: €25-35)
  - Try trdelník (sweet pastry) and roast duck with dumplings

**Weather forecast**: Cooler at 22°C (72°F), chance of thunderstorms

**Day 9: Jewish Quarter and Cultural Sites**
- **Morning**: Visit the Jewish Quarter (Josefov) and synagogues
- **Afternoon**: Municipal House and Powder Tower
- **Evening**: Traditional Czech folklore dinner with performance (Budget: €35-45)
  - Try Czech beer and kolache pastries

**Weather forecast**: Pleasant at 24°C (75°F), mostly sunny

**Day 10: Departure**
- **Morning**: Last-minute shopping in Prague's boutiques
- **Afternoon**: Transfer to airport for departure flight

**Weather forecast**: Pleasant at 24°C (75°F), mostly sunny

#### Budget Breakdown (Per Person)

**Accommodation (9 nights)**
- Paris: €120/night × 3 nights = €360
- Rome: €100/night × 3 nights = €300
- Prague: €80/night × 3 nights = €240
- **Total accommodation**: €900

**Transportation**
- International flights to/from Europe: €600-800 (varies by origin)
- Paris to Rome flight: €100-150
- Rome to Prague flight: €100-150
- Local transportation (metro, bus, tram): €15/day × 10 days = €150
- **Total transportation**: €950-1,250

**Attractions & Activities**
- Paris museums and attractions: €100
- Rome museums and attractions: €80
- Prague museums and attractions: €70
- **Total attractions**: €250

**Food & Dining**
- Breakfast: €10/day × 10 days = €100
- Lunch: €15/day × 10 days = €150
- Dinner: €35/day × 10 days = €350
- Snacks and drinks: €10/day × 10 days = €100
- **Total food**: €700

**Miscellaneous**
- Travel insurance: €50
- Souvenirs and shopping: €200
- Contingency fund: €150
- **Total miscellaneous**: €400

**Grand Total**
- **€3,200-3,500** per person (excluding international flights to/from Europe)

#### UNESCO World Heritage Sites Included
- Palace and Park of Versailles (France)
- Historic Centre of Rome (Italy)
- Vatican City (Italy)
- Historic Centre of Prague (Czech Republic)

#### Travel Tips
1. **Weather**: Based on forecasts, pack for warm weather in all destinations, with temperatures ranging from 20-35°C (68-95°F). Bring a light jacket for cooler evenings in Prague.
2. **Currency**: Euros (€) for France and Italy, Czech Koruna (CZK) for the Czech Republic.
3. **Transportation**: Purchase metro/public transport passes in each city to save money.
4. **Reservations**: Book major attractions in advance to avoid long lines, especially the Louvre, Vatican Museums, and Eiffel Tower.
5. **Water**: Carry a refillable water bottle, especially in Rome where temperatures will be hot.
6. **Language**: Learn a few basic phrases in each language, though English is widely spoken in tourist areas.

This itinerary offers a perfect blend of history, culture, and cuisine across three distinct European regions. The weather should be excellent for sightseeing, with mostly sunny days and warm temperatures. Enjoy your European adventure!

</details>

:::

## Learn More
Check out the [Recipes](/docs/guides/recipes) guide for more docs, tools, and resources to help you master goose recipes.