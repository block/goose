# Using `.goosehints` in Goose

`.goosehints` are text files used within the Goose environment to provide additional context about your project and improve the communication between the developer and Goose. The use of `goosehints` ensures that Goose understands your requirements better and can execute tasks more effectively.

>[!TIP]
> **Developer toolkit required**
>
> To make use of the hints file, you need to have the `developer` toolkit [enabled](https://block.github.io/goose/plugins/using-toolkits.html).

This guide will walk you through creating and using your `.goosehints` file to streamline your workflow with custom instructions and context.

## Creating your `.goosehints` file
You can place a `.goosehints` file in your current working directory or globally at `~/.config/goose/.goosehints`. This file can include any repeated instructions or contextual details relevant to your projects.

A good time to consider adding a `.goosehints` file is when you find yourself repeating prompts, or providing the same kind of instructions multiple times.

### Setting up hints

The `.goosehints` file supports natural language and also follows [jinja templating rules][jinja-guide], so you can leverage templating to insert file contents or variables.

Here are some ways people have used hints to provide additional context for Goose to follow:

- **Decision-Making**: Specify if Goose should autonomously make changes or confirm actions with you first.

- **Validation Routines**: Provide test cases or validation methods that Goose should perform to ensure changes meet project specifications.

- **Feedback Loop**: Include steps that allow Goose to receive feedback and iteratively improve its suggestions.

- **Point to more detailed documentation**: Indicate important files like `README.md`, `CONTRIBUTING.md`, or others that Goose should consult for detailed explanations.

Like prompts, this is not an extensive list to shape your `.goosehints` file. You can include as much context as you need.

Example `.goosehints file`:

```jinja
This is a simple example JavaScript web application that uses the Express.js framework. View [Express documentation](https://expressjs.com/) for extended guidance.

Go through the README.md for information on how to build and test it as needed.

Make sure to confirm all changes with me before applying.

Use the following custom values when needed:
&#123;% include custom-config.js %&#125;

Run tests with `npm run test` ideally after each change.
```

## Best Practices

- **Keep It Updated**: Regularly update the `.goosehints` file to reflect any changes in project protocols or priorities.
- **Be Concise**: Make sure the content is straightforward and to the point, ensuring Goose can quickly parse and act on the information.


[jinja-guide]: https://jinja.palletsprojects.com/en/3.1.x/