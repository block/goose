---
sidebar_position: 1
title: Managing Sessions
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Managing Goose Sessions

A session is a single, continuous interaction between you and Goose, providing a space to ask questions and prompt action. In this guide, we'll cover how to start, exit, save, and resume. 


## Start Session 

<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        From your terminal, navigate to the directory from which you'd like to start, and run:
        ```sh
        goose session 
        ```
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
        After choosing an LLM provider, you’ll see the session interface ready for use.
        
        Type your questions, tasks, or instructions directly into the input field, and Goose will immediately get to work. 

    </TabItem>
</Tabs>

## Name Session
<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        By default, Goose will provide a random string as the name of your session. If you'd like to provide a specific name, this is where you'd do so. For example to name your session `react-migration`, you would run:

        ```
        goose session -n react-migration
        ```

        You'll know your session has started when your terminal looks similar to the following:

        ```
        starting session | provider: openai model: gpt-4o
        logging to ~/.config/goose/sessions/react-migration.json1
        ```
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
        Session management features, such as **naming** and **resuming** sessions, are **not** currently available in the Goose Desktop. If you'd like to see these features added, please [open an issue on GitHub](https://github.com/block/goose/issues/new?template=Blank+issue).
    </TabItem>
</Tabs>
:::info
    If this is your first session, Goose will prompt you for an API key to access an LLM (Large Language Model) of your choice. For more information on setting up your API key, see the [Installation Guide](/docs/getting-started/installation#set-up-a-provider). Here is the list of [supported LLMs](/docs/getting-started/providers).
:::

## Exit Session

<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        To save and exit a session, hold down `Ctrl` + `C`. Alternatively, you can type `exit` to save and exit the session.

        Your session will be stored locally in `~/.config/goose/sessions`.
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
    To exit a session, simply close the application.
    </TabItem>    

</Tabs>

## Resume Session

<Tabs>
    <TabItem value="cli" label="Goose CLI" default>
        To resume your latest session, you can run the following command:

        ```
         goose session -r
        ```

        To resume a specific session, you can first check the sessions you currently have by running: 

        ```
        goose session --list 
        ```

        This command will display a list of all saved sessions, showing a name, date, and time for each session. The output should look similar to the following: 

        ```
        2024-11-12  14:12:28  managing-goose
        2024-11-12. 13:48:11  blog
        2024-11-12  13:27:21  react-migration
        2024-11-04  16:14:29  e6d7
        ```

        To resume a specific session, run the following command: 

        ```
        goose session -r -n react-migration
        ```
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
        Session management features, such as **naming** and **resuming** sessions, are **not** currently available in the Goose Desktop. If you'd like to see these features added, please [open an issue on GitHub](https://github.com/block/goose/issues/new?template=Blank+issue).
    </TabItem>
</Tabs>
