---
title: Google Drive Extension
description: Add Google Drive MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

This tutorial covers how to add the [Google Drive MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/gdrive) as a Goose extension, allowing you to list, read, and search files in Google Drive.

:::tip TLDR

**Command**
```sh
npx -y @modelcontextprotocol/server-gdrive
```

**Environment Variables**
```
GDRIVE_CREDENTIALS_PATH: /Users/<your-username>/servers/.gdrive-server-credentials.json
```
```
GDRIVE_OAUTH_PATH: /Users/<your-username>/servers/gcp-oauth.keys.json
```
:::

## Configuration

To obtain your Google Drive server credentials and oauth keys, follow the steps below:

  1. Set up your Google Cloud Credentials, to enable API access:
        - Create Google Cloud Project
            - Go to [Google Cloud Console](https://console.cloud.google.com/projectcreate) and create a new project
            - You can leave `location` as `No organization`
        - Enable Google Drive API
            - In your project, go to the [API Product Library`](https://console.cloud.google.com/workspace-api/products)
            - Confirm you're in the right project by checking the top left corner
            - Search `Google Drive API` and enable it

  2. Configure OAuth Consent Screen 
        -  Go to the [OAuth Consent Screen](https://console.cloud.google.com/auth/overview/create)
        -  Enter required information, `project name` , `user support email` 
        -  Choose `Internal` for `Audience` and press `create`
        -  If you are unable to choose `Internal` select `External` and follow these additional steps:
            - Navigate to the [Audience](https://console.cloud.google.com/auth/audience) screen
            - Under `Test users` click `Add Users`

 3. Create OAuth Credential
        -  Go to [OAuth Clients](https://console.cloud.google.com/apis/credentials/oauthclient)
        -  Click `Create Client` 
        -  Choose **Application Type: Desktop App**
        -  Download the JSON key file
        -  Rename it to `gcp-oauth.keys.json`
        -  Move it to the root of your cloned MCP repository:
            ```sh
            mv ~/Downloads/gcp-oauth.keys.json ~/servers/gcp-oauth.keys.json
            ```
 
<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
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

  3. Give your extension a name
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
    │  google drive
    // highlight-end
    └ 
  ```

  4. Enter the command
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
    │  google drive
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive
    // highlight-end
    └ 
  ```  

  5. Add your environment variables 

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
    │  google drive
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  GDRIVE_CREDENTIALS_PATH
    │
    ◇  Environment variable value:
    │  /Users/<your-username>/servers/.gdrive-server-credentials.json
    │
    ◇  Add another environment variable?
    │  yes 
    |
    ◇  Environment variable name:
    │  GDRIVE_OAUTH_PATH
    │
    ◇  Environment variable value:
    │  /Users/<your-username>/servers/gcp-oauth.keys.json
    │
    // highlight-end
    └  Added google drive extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click `...` in the upper right corner
  2. Click `Settings`
  3. Under `Extensions`, click the `Add` link
  4. On the `Add Extension Manually` modal, enter the following:
        * **Type**: `Standard IO`
        * **ID**: `g-drive-mcp` (_set this to whatever you want_)
        * **Name**: `google drive` (_set this to whatever you want_)
        * **Description**: `Google Drive MCP Server` (_set this to whatever you want_)
        * **Command**: `npx -y @modelcontextprotocol/server-gdrive`
        * **Environment Variables**
            * **Name**: `GDRIVE_CREDENTIALS_PATH`
            * **Value**: `/Users/<your-username>/servers/.gdrive-server-credentials.json`
            * Click `Add` button
        * **Environment Variables**
            * **Name**: `GDRIVE_OAUTH_PATH`
            * **Value**: `/Users/<your-username>/servers/gcp-oauth.keys.json`
            * Click `Add` button
        
  5. Click `Add Extension` button
  </TabItem>
</Tabs>

## Example Usage

In this example I'm going to have goose list my most recent google drive files, open my Healthy Recipes file, summarize it and tell me the macros for the first meal.

### Goose Prompt
```
list my 3 most recent google drive files
```

### Goose Output

```
Your 3 most recent Google Drive files are:

1. Healthy Recipes
2. Notarized Letter
3. Hit List + Networking Example (MAKE A COPY - DO NOT EDIT)

Would you like me to summarize the "Healthy Recipes" file?

```

### Goose Prompt

```
yes please and tell me the macros of the first meal 
```
### Goose Output
```
The "Healthy Recipes" file contains a recipe for "Sheet-Pan Chicken Fajita Bowls". Here’s a summary:

Ingredients

    - Chili powder, ground cumin, salt, garlic powder, smoked paprika, ground pepper
    - Olive oil
    - Chicken tenders
    - Yellow onion, red bell pepper, green bell pepper
    - Kale
    - Black beans (no-salt-added)
    - Low-fat plain Greek yogurt
    - Lime juice
    - Water

Directions

    1. Preheat oven and prepare a spice mixture.
    2. Toss chicken, onion, and peppers in the spice mixture.
    3. Roast on a baking sheet.
    4. Mix kale and black beans with olive oil and salt, add to the sheet, and roast further.
    5. Prepare a yogurt dressing with the reserved spice mix, lime juice, and water.
    6. Serve the roasted mixture in bowls topped with the yogurt dressing.

To calculate the macro-nutrients (macros) for the "Sheet-Pan Chicken Fajita Bowls," we need to consider the main components:

Estimated Macros for One Serving (approximate):

Since the recipe is divided into 4 servings, I'll provide the macros per serving based on typical ingredient data. Note these are estimates, and actual values may vary:

1. Protein:

    - Chicken tenders (~1.25 lbs or 20 oz total) - approx. 120g protein total, so 30g protein per serving.

2. Carbohydrates:

    - Black beans (15 oz can) - approx. 45g carbs total, so about 11g carbs per serving.
    - Bell peppers and onion add minimal carbs, approximately 5g per serving.

3. Fats:

    - Olive oil (~2 tbsp total) - approx. 28g fat total, so 7g fat per serving.

Total (Per Serving):
    - Protein: ~30g
    - Carbohydrates: ~16g
    - Fats: ~7g

Additionally, the kale and yogurt add dietary fiber and minor macro contributions. If you require precise calculations, using a digital nutrition calculator or app with input from this specific recipe would yield the best results, especially accounting for exact brand and preparation variances.
```