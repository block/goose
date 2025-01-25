---
title: Using Goosehints
sidebar_position: 3
---

# Providing Hints to Goose

`.goosehints` is a text file used to provide additional context about your project and improve the communication with Goose. The use of `goosehints` ensures that Goose understands your requirements better and can execute tasks more effectively.

:::info Developer extension required
To make use of the hints file, you need to have the `developer` extension [enabled](../configuration/using-extensions).

:::

This guide will walk you through creating and using your `.goosehints` file to streamline your workflow with custom instructions and context.

## Creating your hints file

Create a file named `.goosehints` and save the file in `~/.config/goose/.goosehints`. If saved here, Goose will use this file for every session with you.

:::tip
You can also save `.goosehints` local to any directory. In this case, Goose will utilize the hints when working in that directory.
:::

The `.goosehints` file can include any instructions or contextual details relevant to your projects.

A good time to consider adding a `.goosehints` file is when you find yourself repeating prompts, or providing the same kind of instructions multiple times. It's also a great way to provide a lot of context which might be better suited in a file.

## Setting up hints

The `.goosehints` file supports natural language and also follows [jinja templating rules][jinja-guide], so you can leverage templating to insert file contents or variables.

Here are some ways people have used hints to provide additional context to Goose:

- **Decision-Making**: Specify if Goose should autonomously make changes or confirm actions with you first.

- **Validation Routines**: Provide test cases or validation methods that Goose should perform to ensure changes meet project specifications.

- **Feedback Loop**: Include steps that allow Goose to receive feedback and iteratively improve its suggestions.

- **Point to more detailed documentation**: Indicate important files like `README.md`, `CONTRIBUTING.md`, or others that Goose should consult for detailed explanations.

Like prompts, this is not an extensive list to shape your `.goosehints` file. You can include as much context as you need.

Example `.goosehints` file:

```jinja
This is a simple example JavaScript web application that uses the Express.js framework. View [Express documentation](https://expressjs.com/) for extended guidance.

Go through the README.md for information on how to build and test it as needed.

Make sure to confirm all changes with me before applying.

Use the following custom values when needed:
{%include custom-config.js%}

Run tests with `npm run test` ideally after each change.
```

## Best practices

- **Keep file updated**: Regularly update the `.goosehints` file to reflect any changes in project protocols or priorities.
- **Be concise**: Make sure the content is straightforward and to the point, ensuring Goose can quickly parse and act on the information.


[jinja-guide]: https://jinja.palletsprojects.com/en/stable/