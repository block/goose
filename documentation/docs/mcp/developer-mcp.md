---
title: Developer Extension
description: Use Developer MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/on_p-LeIrak" />

The Developer extension allows Goose to automate developer-centric tasks such as file editing, shell command execution, and project setup.

This tutorial will cover enabling and using the Developer MCP Server, which is a built-in Goose extension. 


## Configuration

:::info
The Developer extension is already enabled by default when Goose is installed.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Developer"
    description="Automate developer-centric tasks like file editing and shell commands"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   goose-configure 
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

In this example, I'm going to have Goose automate setting up my JavaScript developer environment with Express, Mongoose, Nodemon, Dotenv and initialize Git.

:::info LLM
Anthropic's Claude 3.5 Sonnet was used for this task.
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
   1. Open a new session in Goose Desktop
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Open a terminal and start a new Goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### Goose Prompt
```
set up a new JavaScript project with Express, Mongoose, Nodemon, and Dotenv? Fetch the latest package versions, generate a README with those versions, and initialize Git
```

After 1 minute, Goose was done setting up my entire dev environment! 🎉

### Goose Output

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
2. Installed Express, Mongoose, and dotenv dependencies
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