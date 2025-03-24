# Experimental Features

Goose is an open source project that is constantly being improved, and new features are added regularly. Some of these features are considered experimental, meaning they are still in development and may not be fully stable or ready for production use. This guide covers how to enable and use experimental features in Goose, as well as how to provide feedback on them.

:::note
Experimental features are new capabilities that are still being tested and refined. While they can provide additional functionality, they may be less stable than standard features.
:::

## Enabling Experimental Features

To access experimental features, run:

```sh
goose configure
```

Select "Goose Settings" from the menu:

```sh
┌ goose-configure
│
◆ What would you like to configure?
| ○ Configure Providers
| ○ Add Extension
| ○ Toggle Extensions
| ○ Remove Extension
// highlight-next-line
| ● Goose Settings (Set the Goose Mode, Tool Output, Experiment and more)
└
```

Then select "Toggle Experiment" option in the menu:
   ```sh
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Goose Settings 
   │
   ◆  What setting would you like to configure?
   │  ○ Goose Mode 
   │  ○ Tool Output 
   // highlight-next-line
   │  ● Toggle Experiment (Enable or disable an experiment feature)
   └  
   ```

## Available Experimental Features

:::note
There are no experimental features at this time!
:::

:::note
The list of experimental features may change as Goose development progresses. Some features may be promoted to stable features, while others might be modified or removed.This section will be updated with specific experimental features as they become available
:::

## Feedback

If you encounter any issues with these features, check if the issue is already reported in the [GitHub issues](https://github.com/goose/goose/issues) or join the [Discord community](https://discord.gg/block-opensource) to share.