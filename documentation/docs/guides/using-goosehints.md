---
title: Providing Hints to Goose
sidebar_position: 4
sidebar_label: Using Goosehints
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { FolderKey } from 'lucide-react';

`.goosehints` is a text file used to provide additional context about your project and improve the communication with Goose. The use of `goosehints` ensures that Goose understands your requirements better and can execute tasks more effectively.

<details>
  <summary>Goose Hints Video Walkthrough</summary>
  <iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/kWXJC5p0608"
  title="Goose Hints"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
  allowFullScreen
  ></iframe>
</details>

:::info Developer extension required
To make use of the hints file, you need to have the `Developer` extension [enabled](/docs/getting-started/using-extensions).

:::

This guide will walk you through creating and using your `.goosehints` file to streamline your workflow with custom instructions and context.

## Creating your hints file

Goose supports two types of hint files:
- **Global hints file** - These hints will apply to all your sessions with Goose, regardless of directory.
- **Local hints file** -  These hints will only apply when working in a specific directory.

:::tip
You can use both global and local hints at the same time. When both exist, Goose will consider both your global preferences and project-specific requirements. If the instructions in your local hints file conflict with your global preferences, Goose will prioritize the local hints.
:::

:::tip
You can name your context files differently -- e.g. `AGENTS.md` -- and Goose can still pick them up. Configure the `CONTEXT_FILE_NAMES` setting!
:::

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>

    #### Global hints file
    1. Create a `.goosehints` file in `~/.config/goose`.

     #### Local hints file

    1. Change the directory to where you'd like to set up the file. You can do this by clicking the directory path on the bottom of the Goose window.
    2. Click the <FolderKey size={16} /> icon on the bottom right of the Goose window.
    4. Enter your local tips into the text area.
    5. Click `Save`.
    6. Restart your session so Goose can read the updated `.goosehints`.

    If a `.goosehints` file already exists in the given directory, you can edit or add to it from this screen.

    :::tip
    You may have to scroll or adjust the screen size to fully see the Save and Cancel buttons.
    :::

    </TabItem>
    <TabItem value="manual" label="Manual">
    
    - **Global hints file** - Create a `.goosehints` file in `~/.config/goose`.
    - **Local hints file** -  Create a `.goosehints` file at the root of the directory you'd like it applied to.

    </TabItem>
</Tabs>



The `.goosehints` file can include any instructions or contextual details relevant to your projects.

A good time to consider adding a `.goosehints` file is when you find yourself repeating prompts, or providing the same kind of instructions multiple times. It's also a great way to provide a lot of context which might be better suited in a file.

Goosehints are loaded at the start of your session and become part of the system prompt sent with every request. This means the content of `.goosehints` contributes to token usage, so keeping it concise can save both cost and processing time.

## Setting up hints

The `.goosehints` file supports natural language.

### Example global `.goosehints` file

```
Always use TypeScript for new Next.js projects.

Follow the [Google Style Guide](https://google.github.io/styleguide/pyguide.html) for Python code.

Run unit tests before committing any changes.

Prefer functional programming patterns where applicable.
```

### Example local `.goosehints` file

```
This is a simple example JavaScript web application that uses the Express.js framework. View [Express documentation](https://expressjs.com/) for extended guidance.

Go through the README.md for information on how to build and test it as needed.

Make sure to confirm all changes with me before applying.

Run tests with `npm run test` ideally after each change.
```

## Common use cases
Here are some ways people have used hints to provide additional context to Goose:

- **Decision-Making**: Specify if Goose should autonomously make changes or confirm actions with you first.

- **Validation Routines**: Provide test cases or validation methods that Goose should perform to ensure changes meet project specifications.

- **Feedback Loop**: Include steps that allow Goose to receive feedback and iteratively improve its suggestions.

- **Point to more detailed documentation**: Indicate important files like `README.md`, `CONTRIBUTING.md`, or others that Goose should consult for detailed explanations.

Like prompts, this is not an extensive list to shape your `.goosehints` file. You can include as much context as you need.

## Best practices

- **Keep file updated**: Regularly update the `.goosehints` file to reflect any changes in project protocols or priorities.
- **Be concise**: Make sure the content is straightforward and to the point, ensuring Goose can quickly parse and act on the information.
- **Start small**: Create a small set of clear, specific hints and gradually expand them based on your needs. This makes it easier to understand how Goose interprets and applies your instructions.
**Reference other files**: Point Goose to relevant files like /docs/style.md or /scripts/validation.js to reduce repetition and keep instructions lightweight.
