---
sidebar_position: 5
title: Making Shared Goose Agents
sidebar_label: Share Sessions
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


A shared session in Goose is like a collaborative workspace where multiple people can work with the AI assistant together in real-time. Think of it similar to a shared Google Doc, but for AI assistance.

## Create a shared Agent
When you create an Agent to be shared, you are creating a host Agent. When your host terminates, the shared Agents are disconnected and collaborators can no longer see activity in the host.
<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        The Goose CLI can generate a "recipe" which encapsulates all of the components to be shared with another Goose user. To share an agent, first create a recipe file.

        In your terminal with a Goose session running, input the following:
        ```sh
        ( O)> /recipe 
        ```
        Goose generates `recipe.yaml` and saves it for you as shown in the following output:

        ```sh
        Generating Recipe
        Saved recipe to .../my-shared-project/goose/recipe.yaml
        ```
        Send the file to the Goose CLI user that wants to start a shared session.  For more information about the `recipe` prompt completion command, see the [recipe](/docs/guides/goose-cli-commands#recipe) CLI prompt.
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">

        To share a session at any time, click the three dots in the top-right corner of the application and select **Make Agent from this session** from the dropdown menu. 

        Goose gathers the components to be shared and opens a dialog that provides an Agent URL to be provided to the Goose user who will share your session. Send the URL to the people who are going to collaborate with you. Their browser will create a new shared Agent in Goose when it reads the URL in the address bar.


    </TabItem>
</Tabs>

## Start the shared Agent 

<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        When another Goose CLI user sends you a recipe file, save it in the directory where you want to start your shared Agent.
        From your terminal, navigate to the directory where you saved the file, and run:
        ```sh
        goose run --<RECIPE_FILE_NAME> 
        ```
        The Goose CLI loads the recipe and creates a session with all of the components shared in the original session. For  infomration about the Goose `run` command, see the [run](/docs/guides/goose-cli-commands#run-options) CLI command.

    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
        When another Goose desktop user wants to share an Agent, they send you an URL which you use to start the new shared Agent.

        Open a new tab on your browser and paste the shared Agent URL into the address bar and press the **enter** key on your keyboard. The browser requests your permission to start a new Goose session with the shared components. 

        The new Goose session shows a set of actions that you can take as clickable buttons. As shown in the following image, the shared Agent is shown as an action that summarizes the purpose of the shared Agent. To activate the agent, review the summary and then select the action.  

  
    </TabItem>
</Tabs>

## Accessing conversation history in shared sessions
When you join a shared session, you automatically get access to:
* The full conversation history (chat messages) from when the session started
* All tool outputs and results
* Any files or content created during the session

### How It Works
When you click the shared link, you'll join the active session. The conversation history will automatically load in your Goose window. You'll see all messages and interactions in real-time and can scroll up to view earlier parts of the conversation.
### Important Notes
The history is synchronized live - you'll see new messages as they happen. You don't need to take any special steps to access the history. The conversation remains available as long as the session is active. Once the host ends the session, the shared access ends.

### Troubleshooting
If you're having trouble accessing the conversation history when joining a shared session, you might want to:
* Make sure you're using the most recent version of Goose
* Try refreshing your session
* Check with the host to ensure the session is still active

Remember that the shared session ends when the host closes it, so make sure to save any important information you need before the session ends.

## Tool outputs in a shared session
In a shared session, participants can see:
* All conversation messages
* Tool outputs and results
* Files or content created during the session
* Active extensions and their configurations
### How tool outputs work in shared sessions
When any participant uses a tool, all members can see:
* The tool being called
* The parameters used
* The results/output of the tool

These outputs appear in the conversation just like messages and they're synchronized in real-time for all participants.

### Important Notes
* Tool outputs are treated as part of the conversation history
* All participants can see the results, even if they didn't initiate the tool use
* The outputs remain visible as long as the session is active
* Like other shared content, tool outputs are only available during the active session

This means that when you're in a shared session, you have full visibility into all tool interactions and their results, making it effective for collaborative troubleshooting or working together on tasks that require tool use.

## File access in shared sessions
Files created during a session are meant to be accessible to all participants. However, a file that is created in one shared Agent instance is not created in parallel on the other instances. To give everyone access to the file, we suggest any of the following strategies:

* **Ask the Host**: When a collaborator creates a file, ask them to:
   1. Share the file's location or path
   1. Confirm how they intend to share access to the file

* **Use Shared Tools**: When files need to be shared:
   1. Use collaborative tools like Google Drive (if the extension is available)
   1. Share file contents directly in the conversation where possible
   1. Consider using version control systems for code files

* **Document Important Files**:
   1. Keep track of important files created during the session
   1. Save or copy relevant content before the session ends


## What gets shared?
You may have started a project in a Goose session that needed information that your collaborating team member needs in the shared agent that you create. At the same time, you may have needed to give Goose information that you'd rather not share. 
### Shared components
The shared agent includes these components:
* Conversation history (all messages)
* Tool outputs and results
* Files or content created during the session
* Active extensions and their configurations
* Project context (when working within a project)

### Private components
The following components are not included in a shared agent:
* Global memories (stored in `~/.config/goose/memory`)
* Local memories (stored in .goose/memory)
* Personal API keys or credentials
* System-level configurations



## Common use cases
There are many reasons why you might want to create a shared agent. The following shared agent use  cases are just a starting point.

### Team Collaboration
* Working together on coding projects
* Troubleshooting technical issues
* Brainstorming sessions
* Training & Onboarding

### Teaching new team members
* Demonstrating workflows
* Sharing best practices
* Pair Programming

### Real-time code collaboration
* Code reviews
* Debugging sessions





