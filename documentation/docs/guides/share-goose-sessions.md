---
sidebar_position: 5
title: Sharing a Goose Agent
sidebar_label: Share Goose Agents
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


A shared Agent in Goose is like a collaborative workspace where multiple people can work with the AI assistant together in real-time. Think of it similar to a shared Google Doc, but for AI assistance.

## Create a shared Agent
When you create an Agent to be shared, you are creating a host Agent. When your host terminates, the shared Agents are disconnected and collaborators can no longer see activity in the host.

### Setup recipe
To create a custom session (whether using the desktop app or CLI), you'll need:

* Instructions for the AI
* Activities it should perform
* A custom prompt (optional)

The CLI calls this a "recipe." The desktop app shows these same options in the Agent Created dialog when you create a new shared session.

:::tip 
Both tools need the same information, they just use different names for it.
:::

### Agent setup instructions
When you share your Goose Agent setup (either through the command line or desktop app), you'll get a chance to review and edit the setup instructions. These instructions come from your current session, but they might need some extra details to work well for others.

Here's an example: Let's say you were working with Goose to make debug a new application and you need help with the debug logs. In this case, you should add a note like this to your instructions:

"I asked Goose to review the debug logs from this application: https://github.com/square/connect-api-examples/tree/master/connect-examples/v2/node_orders-payments. The log files are in this folder:  https://github.com/square/connect-api-examples/blob/master/my_app/v2/logs/"

 
This tells the new shared Agent (or new local session) what the context is for your debugging collaboration. The critical context that the shared Agent (or session) needs is the location of the application source code and the folder where log files are written. With this context, the teammate you are working with can ask Goose to clone the GitHub repo, open the project in an IDE, and build the project.  

### Agent setup activities
When you work with Goose, it keeps track of everything you do together. Before sharing your setup with others, you should review this list of activities. Think of it like cleaning up your workspace before inviting colleagues over:

* Look at the list of activities when creating your shared setup
* Pick only the activities that matter for your project
* Remove any activities that aren't relevant for your teammates

For example: If you used Goose for both writing the application UI and helping you debug it, but only want to share the debugging part, you can remove the coding the application activities.




<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        To create a shared Agent using the CLI, you need a recipe. Think of a Goose "recipe" like a blueprint - it contains the instructions to recreate your agent for someone else. Here's what you need to know:

        * A recipe is different from sharing a live agent session
        * The recipe tells Goose how to build a copy of your agent
        * It includes things like:
           * Instructions for how the agent should behave
           * What activities the agent can do

        When you use the Goose desktop app to share an agent, it shows you a window where you can review and customize these settings before creating the recipe. With the CLI, the `/recipe` command creates a .yaml file that you can edit to customize the shared Agent that you want to create. 

        Note: This is like sharing a cooking recipe with a friend - you're not giving them your actual meal, but the instructions to make the same thing themselves!

        In your terminal with a Goose session running, input the following:
        ```sh
        ( O)> /recipe 
        ```
        Goose generates `recipe.yaml` and saves it for you as shown in the following output:

        ```sh
        Generating Recipe
        Saved recipe to .../my-shared-project/goose/recipe.yaml
        ```

        ### Recipe.yaml specification
        The .yaml file contains an editable  set of fields that let you customize the shared Agent that gets created.

        ```yaml
        ---
        # Required fields
        version: 1.0.0
        title: $title
        description: $description

        instructions: $instructions # instructions to be added to the system prompt

        # Optional fields
        prompt: $prompt             # if set, the initial prompt for the run/session

        extensions:
        - $extensions
        context:
        - $context
        activities:
        - $activities
        author:
        contact: $contact
        metadata: $metadata
        ```
        You can edit the `instructions` and `activities` fields. Instructions can be edited to add context that isn't in the session that you are creating the shared Agent from. You can also add additional activites such as "Please ask users for their zipcode so you can provide their local weather forecast." 


        Send the file to the Goose CLI user that wants to start a shared Agent.  For more information about the `recipe` prompt completion command, see the [recipe](/docs/guides/goose-cli-commands#recipe) CLI prompt.
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">

        To share an Agent at any time, 
        1. click the three dots in the top-right corner of the application
        1. select **Make Agent from this session** from the dropdown menu.

        Goose will create a shareable version of your Agent and show you a dialog with:
        * A shareable URL
        * Instructions for the new agent
        * Example activities the agent can help with

        ## Customizing your shared Agent
        Before sharing, you can review and edit how your agent will help others:

        ### Instructions
        These tell the agent how to help other users. Goose creates these automatically based on your conversation, but you can edit them to add more guidance.

        #### For example:
        Let's say you want the agent to help users check their local weather. You could edit the instructions to add:

        **"Anyone who uses this agent to help with the project needs to know what the weather is like now because they need to work outside and stay dry and warm."**

       

        ### Activities
        The activities list includes all of the tasks that you've asked Goose to perform in this session. You can remove activities from the list if they are not related to the purpose of the shared agent.  You can also add new activites such as the following:

        **"Please ask users for their zipcode so you can provide their local weather forecast."**

        ### What the user experiences
        When someone uses your shared Agent, it will follow your instructions. Using our weather example:

        ```
        user: "will it be sunny and warm today?"
        Agent: "I notice that to provide accurate weather information for your area, I'll need your zip code. Could you please provide your zip code so I can check the current weather conditions for your specific location? - This is important since the instructions mention that users of this project need to know the weather conditions as they'll be working outside and need to stay dry and warm.
        
        Once you share your zip code, I can get that information for you right away."
        ```

        Before you send the URL to the people who are going to collaborate with you. Their browser will create a new shared Agent in Goose when it reads the URL in the address bar.


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

        The new Goose session shows a set of activities that you can run. Any prompts that you give the Agent are processed in the context provided by the recipe created by the hosting user.    

  
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
You might start a project in a Goose session and realize your teammate needs access to that context through a shared agent. But at the same time, you may have shared things with Goose that you’d rather keep private.
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





