---
title: Developer Extension
description: Use Developer MCP Server as a gosling Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GoslingBuiltinInstaller from '@site/src/components/GoslingBuiltinInstaller';
import { Tornado } from 'lucide-react';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/on_p-LeIrak" />

The Developer extension allows gosling to automate developer-centric tasks such as file editing, shell command execution, and project setup. It also supports [enhanced code editing](/docs/guides/enhanced-code-editing). For semantic [codebase analysis](/docs/guides/codebase-analysis), see the separate Analyze extension.

This tutorial will cover enabling and using the Developer MCP Server, which is a built-in gosling extension. 


## Configuration

:::info
The Developer extension is already enabled by default when gosling is installed.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
  <GoslingBuiltinInstaller
    extensionName="Developer"
    description="Automate developer-centric tasks like file editing and shell commands"
  />
  </TabItem>
  <TabItem value="cli" label="gosling CLI">

  1. Run the `configure` command:
  ```sh
  gosling configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   gosling-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  Enable extensions: (use "space" to toggle and "enter" to submit)
  // highlight-start    
  │  ● developer
  // highlight-end  
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Example Usage

In this example, I'm going to have gosling automate setting up my JavaScript developer environment with Express, Mongosling, Nodemon, Dotenv and initialize Git.


<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
   1. Open a new session in gosling Desktop
  </TabItem>
  <TabItem value="cli" label="gosling CLI">

  1. Open a terminal and start a new gosling session:

  ```sh
  gosling session
  ```

  </TabItem>
</Tabs>

### gosling Prompt
```
set up a new JavaScript project with Express, Mongosling, Nodemon, and Dotenv? Fetch the latest package versions, generate a README with those versions, and initialize Git
```

After 1 minute, gosling was done setting up my entire dev environment! 🎉

### gosling Output

```md
I'll help you set up a new JavaScript project with those dependencies. I'll break this down into steps:

1. First, let's create a new directory and initialize the project
2. Then install the dependencies
3. Create a detailed README
4. Initialize git
5. Create some starter files
6. Let's begin:

[... shell commands, files created ...]

Great! I've set up a complete Express project with MongoDB and Dotenv integrations:

1. Created a new project directory and initialized npm
2. Installed Express, Mongosling, and dotenv dependencies
3. set up the basic project structure with directories for:
    - config/
    - models/
    - routes/ 
    - controllers/
4. Created essential files:
    - index.js (main server file)
    - .env (environment variables)
    - .ignore (with common node.js ignores)
5. Added a basic user model with basic name and email fields
6. Created a user controller with two methods:
    - getUsers (GET all users)
    - createUser (POST new user)
7. Added user routes under `/api/routes`
8. Updated package.json with start scripts
9. Created a comprehensive README.md with:
    - Installation scripts
    - Project structure
    - Available scripts
    - Environment variables 
    - Latest package versions

The project structure is:

my-express-app/
├── config/         # Configuration files
├── models/        # MongoDB models
│   └── User.js    # User model
├── routes/        # API routes
│   └── userRoutes.js
├── controllers/   # Route controllers
│   └── userController.js
├── .env           # Environment variables
├── .gitignore     # Git ignore file
├── index.js       # Entry point
└── package.json   # Project dependencies and scripts

To start developing you can:

1. Run `npm install` to install dependencies
2. start the server with `npm run dev` 

The API will be available at:
    - `http://localhost:3000/api/users` (GET to get all users)
    - `http://localhost:3000/api/users` (POST to create a new user)

To test out the API you can use tools like postman or curl, for example, to create a new user:

curl x POST http://localhost:3000/api/users \
-H "Content-Type: application/json" \
-d '{"name": "Ebony Louis" , "email": "ebony@example.com"}'

Would you like me to add any additional features or make other improvements to the setup?
```

## Environment Variables in Shell Commands

Shell commands executed by the `shell` tool inherit the environment of the running gosling process. This typically includes:
- System variables like `PATH`, `HOME`, and `USER`
- Environment variables present in the process that launched gosling (for example, your terminal's environment when you start gosling from a shell)
- Session-specific variables injected by gosling, such as `AGENT_SESSION_ID` for [session-isolated workflows](/docs/guides/environment-variables#using-session-ids-in-workflows)

This enables workflows that depend on environment configuration, such as authenticated CLI operations and build processes.

:::info
gosling Desktop or launcher-based starts may use a different environment and may not load your shell startup files.
:::

:::warning Sensitive Information
Environment variables may contain sensitive values like API keys and tokens (e.g., `GITHUB_TOKEN`, `AWS_ACCESS_KEY_ID`).
:::

## Configuring Access Controls

By default, gosling can run system commands with your user privileges and edit any accessible file **without your approval**. This is because gosling runs in Autonomous permission mode by default and has access to the Developer extension's shell and file editing tools. While this configuration allows gosling to work quickly and independently, there's potential for unexpected outcomes. Understanding the available access control features can help you configure gosling to match your comfort level and specific needs.

:::tip
See the [Quick Setup Example](#quick-setup-example) below for some ways to configure more control over gosling's behavior.
:::

### Developer Extension Tools

The Developer extension provides these tools:

| Tool | Description | Use Cases | Risk Level |
|------|-------------|-----------|------------|
| `shell` | Execute shell commands | Running tests, installing packages, git operations | ⚠️ High<br />Can run any system command with your user privileges |
| `write` | Create or overwrite files | Creating new files, generating code, writing configs | ⚠️ High<br />Can create or replace any accessible file |
| `edit` | Find-and-replace edits within a file | Code refactoring, updating configs, targeted fixes | ⚠️ High<br />Can modify any accessible file |
| `tree` | List a directory tree with line counts | Exploring project structure, locating files | ✅ Low<br />Read-only directory listing |
| `read_image` | Read an image file as visual content | Inspecting screenshots, diagrams, UI assets | ✅ Low<br />Read-only image access |

### Access Control Features

You can layer multiple controls to match your risk tolerance and workflow:

- **[gosling Permission Modes](/docs/guides/managing-tools/gosling-permissions)** control when gosling asks for approval:

  | Mode | Description | Use Cases |
  |------|-------------|-----------|
  | Autonomous<br />CLI: `auto` | No approval required | Best for experienced users in safe environments |
  | Manual Approval<br />CLI: `approve` | Review every action | Recommended for sensitive work or when you want maximum control |
  | Smart Approval<br />CLI: `smart_approve` | AI decides what needs review | Balanced approach |
  | Chat Only<br />CLI: `chat` | Disable all tools | For maximum security and models that don't support tool-calling |

- **[Tool Permissions](/docs/guides/managing-tools/tool-permissions)** let you set `Always allow`, `Ask before`, and `Never allow` permissions for individual extension tools when in Manual Approval or Smart Approval modes

- **[.goslingignore files](/docs/guides/context-engineering/using-goslingignore)** restrict which files and directories gosling can access (`.gitignore` files are fallback)

:::tip Changing Modes In-Session
You can change gosling permission modes during a session without restarting:
- **CLI**: Use the `/mode` command (e.g. `/mode approve`)
- **Desktop**: Use the <Tornado className="inline" size={16} /> mode selector button in the bottom menu
:::

#### Quick Setup Example

You might want more control over gosling's operations when working with sensitive systems, exploring unfamiliar codebases, using untrusted models, or simply preferring to review actions before execution.

Here's an example configuration that enables oversight:

1. **Set the [permission mode](/docs/guides/managing-tools/gosling-permissions)** to Smart Approval or Manual Approval:
   ```yaml
   # ~/.config/gosling/config.yaml
   GOSLING_MODE: smart_approve  # or approve
   ```

2. **Create a [`.goslingignore` file](/docs/guides/context-engineering/using-goslingignore)** in your project to protect sensitive files:
   ```
   .env*
   secrets.*
   *.key
   *.pem
   .git/
   ```

3. **Configure [tool permissions](/docs/guides/managing-tools/tool-permissions)** based on your needs

As you become more comfortable with gosling's behavior, you can adjust these settings to reduce friction while maintaining appropriate safeguards for your environment.

:::info
Also see the [Security Guide](/docs/guides/security/) for information about using gosling safely.
:::

## Additional Resources

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';

<ContentCardCarousel
  items={[
    {
      type: 'topic',
      title: 'Enhanced Code Editing with AI Models',
      description: 'Use AI models to intelligently apply code changes',
      linkUrl: '/docs/guides/enhanced-code-editing'
    },
    {
      type: 'topic',
      title: 'Codebase Analysis',
      description: 'Understand codebases with semantic analysis and call graphs',
      linkUrl: '/docs/guides/codebase-analysis'
    }
  ]}
/>
