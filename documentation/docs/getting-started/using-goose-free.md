---
sidebar_position: 1
title: Using Goose for Free
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Using Goose for Free

Goose is a free and open-source developer agent that you can start using right away, but not all supported [LLM Providers][providers] provide a free tier. 

Below, we outline a couple of free options and how to get started with them.


## Google Gemini
Google Gemini provides free access to its AI capabilities with some limitations. To start using the Gemini API with Goose, you need an API Key from [Google AI studio](https://aistudio.google.com/app/apikey).

To set up Google Gemini with Goose, follow these steps:

<Tabs>
  <TabItem value="cli" label="Goose CLI" default>
    1. Run: 
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose `Google Gemini` as the provider and enter your API key. 

    ![Gemini Config](../assets/guides/gemini-config-cli.png)
    
  </TabItem>
  <TabItem value="ui" label="Goose UI">
  **To update your LLM provider and API key:** 

    1. Click on the three dots in the top-right corner.
    2. Select `Provider Settings` from the menu.
    2. Choose `Google Gemini` as provider from the list.
    3. Click Edit, enter your API key, and click `Set as Active`.

  </TabItem>
</Tabs>

## Limitations

These free options are a great way to get started with Goose and explore its capabilities. However, if you need more advanced features or higher usage limits, you can always upgrade to a paid plan.

---

This guide will continue to be updated with more free options as they become available. If you have any questions or need help with a specific provider, feel free to reach out to us on [Discord](https://discord.gg/block-opensource) or on the [Goose repo](https://github.com/block/goose).


[providers]: https://block.github.io/goose/plugins/providers.html