---
title: Memory Extension
description: Use Memory MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';


The Memory extension allows Goose to store, organize, and retrieve important information (like commands, code snippets, and configurations) across conversations, with support for both project-specific (local) and universal (global) knowledge management.

This tutorial will cover enabling and using the Memory MCP Server, which is a built-in Goose extension. 

## Configuration

1. Ensure extension is enabled:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Built-in Extension`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension 
  │
  ◆  What type of extension would you like to add?
  // highlight-start    
  │  ● Built-in Extension (Use an extension that comes with Goose)
  // highlight-end  
  │  ○ Command-line Extension 
  │  ○ Remote Extension 
  └  
  ```

  3. Arrow down to the `Memory` extension and press Enter
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◆  Which built-in extension would you like to enable?
  │  ○ Developer Tools 
  │  ○ Computer Controller 
  // highlight-start
  │  ● Memory 
  // highlight-end
  |  ○ JetBrains
  └  Enabled Memory extension
  ```
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click `...` in the upper right corner
  2. Click `Settings`
  3. Under `Extensions`, toggle `Memory` to on.
  </TabItem>
</Tabs>

## Example Usage

In this example, I'm going to have Goose create a personal knowledge base for me. Instead of trying to remember everything on your own, Goose can store any form of information for you from commands, local or global project knowledge, preferences, etc.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

  1. Open a terminal and start a new Goose session:

  ```sh
  goose session
  ```

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
   1. Open a new session in Goose Desktop
  </TabItem>
</Tabs>

### Goose Prompt #1
```
can you store these for me:

1. This Docker debugging command:
    docker logs -f --tail=100 $(docker ps -q --filter name=api)

2. This Python retry pattern:
    def retry(max_attempts=3, delay=1):
        def decorator(func):
            def wrapper(*args, **kwargs):
                attempts = 0
                while attempts < max_attempts:
                    try:
                        return func(*args, **kwargs)
                    except Exception as e:
                        attempts += 1
                        if attempts == max_attempts:
                            raise e
                        time.sleep(delay)
            return wrapper
        return decorator

3. And save this locally for the current project:
    API Configuration:
    PORT=3000
    NODE_ENV=development
    DATABASE_URL=postgresql://localhost:5432/myapp
    REDIS_URL=redis://localhost:6379

After storing these, show me everything you've saved.
```

### Goose Output

```
I'll help you store these items. Let me handle them one by one.

[ ... storing and organizing ...]

Now, let me show you everything that's been saved. I'll check both global and local memories:

1. Storing Docker command in:
    - Category: commands
    - Tags: #docker #debugging #logs
    - Scope: global

2. Storing Python pattern in:
    - Category: code_snippets
    - Tags: #python #decorator #error_handling #retry
    - Scope: global

3. Storing API configuration in:
    - Category: project_setup
    - Tags: #config #environment #api
    - Scope: local

All items have been stored! Here's everything in your knowledge base:

Commands Category:
    - docker logs -f --tail=100 $(docker ps -q --filter name=api)
    Description: Shows last 100 lines and follows logs for container with 'api' in name

Code Snippets Category:
    - Python retry decorator pattern
    Tags: #python #decorator #error_handling
    (Use "Show me the retry pattern" to see full code)

Project Setup Category (Local):
    - API Configuration
    Tags: #config #environment #api
    (Use "Show me the API config" to see all variables)
```
### Goose Prompt # 2

```
Show me my saved retry pattern and delete my global docker command
```
### Goose Output
```
First, let me show you the stored retry pattern:

Found in Code Snippets Category (#python #decorator #error_handling #retry):

def retry(max_attempts=3, delay=1):
    def decorator(func):
        def wrapper(*args, **kwargs):
            attempts = 0
            while attempts < max_attempts:
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    attempts += 1
                    if attempts == max_attempts:
                        raise e
                    time.sleep(delay)
        return wrapper
    return decorator

Now, removing the Docker command:
Found and removed from Commands Category:
- docker logs -f --tail=100 $(docker ps -q --filter name=api)

Command has been deleted successfully. Your knowledge base now contains:
- Python retry pattern (global)
- API Configuration (local to current project)
```