# Tool Router (preview)

## Overview

Tool Router is a powerful feature that addresses a common challenge in LLM-based development: the difficulty of selecting the right tool when multiple extensions are enabled. Traditional approaches feed an entire list of tools into the context during chat sessions, which not only consumes a significant number of tokens but also reduces the effectiveness of tool calling.

## The Problem

When you enable multiple extensions (like Slack), you get access to numerous tools such as:
- Reading threads
- Sending messages
- Creating channels
- And many more

However, you typically don't need all this functionality at once. Loading every available tool into the context can be inefficient and potentially confusing for the LLM.

## The Solution: Tool Router

Tool Router introduces a smarter way to handle tool selection through vector-based indexing. Instead of passing all tools back and forth, it:

1. Indexes all tools from your enabled extensions
2. Uses vector search to load only the relevant tools into context when needed
3. Ensures that only the functionality you actually need is available

## Configuration

To enable this feature, change the Tool Selection Strategy from default to vector.

#### CLI
To configure Tool Router in the CLI, follow these steps:

1. Run the configuration command:
```bash
./target/debug/goose configure
```

2. This will update your existing config file. Alternatively, you can edit it directly at:
```
/Users/wendytang/.config/goose/config.yaml
```

3. During configuration:
   - Select "Goose Settings"
   - Choose "Router Tool Selection Strategy"
   - Select "Vector Strategy"

The configuration process will look like this:
```
┌   goose-configure
│
◇  What would you like to configure?
│  Goose Settings
│
◇  What setting would you like to configure?
│  Router Tool Selection Strategy
│
◇  Which router strategy would you like to use?
│  Vector Strategy
│
└  Set to Vector Strategy - using vector-based similarity for tool selection
```

#### UI
Toggle the settings button on the top right and head to 'Advanced Settings', then 'Tool Selection Strategy' at the botoom.

## Benefits

- Reduced token consumption
- More accurate tool selection
- Improved LLM performance
- Better context management
- More efficient use of available tools 

## Notes

### Model Compatibility

Tool Router currently only works with Claude models served through Databricks. The embedding functionality uses OpenAI's `text-embedding-3-small` model by default.

### Feedback & Next Steps

We'd love to hear your thoughts on this feature! Please reach out in the Goose Discord channel to share your use case and experience.

Our roadmap includes:
- Expanding Tool Router support to OpenAI models
- Adding customization options for the `k` parameter that controls how many similar tools are returned during vector similarity search
