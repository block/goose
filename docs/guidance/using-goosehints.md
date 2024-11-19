# Using `.goosehints` in Goose

`.goosehints` are text files used within the Goose environment to provide additional context about your project and improve the communication between the developer and Goose. Ensuring it understands your requirements better and can execute tasks more effectively.

>[!TIP]
> **Developer toolkit required**
>
> To make use of the hints file, you need to have the `developer` toolkit enabled.

This guide will walk you through using your `.goosehints` file to streamline your workflow with custom instructions and context.

## Creating your `.goosehints` file
You can place a `.goosehints` file in your current working directory or globally at `~/.config/goose/.goosehints`. This file can include any repeated instructions or contextual details relevant to your projects.

A good time to consider adding a `.goosehints` file is when you find your self repeating prompts, or providing the same kind of instructions multiple times.

The `.goosehints` file follows [jinja templating rules][jinja-guide] in case you want to leverage templating to insert file contents or variables. But you can also add instructions in natural language.

### Setting Rules

Some rules you can define for Goose to follow:
- **Decision-Making**: Specify if Goose should autonomously make changes or confirm actions with you first.
- **Validation Routines**: Provide test cases or validation methods that Goose should perform to ensure changes meet project specifications.
- **Feedback Loop**: Include steps that allow Goose to receive feedback and iteratively improve its suggestions.
- **Point to more detailed documentation**: Indicate important files like `README.md`, `CONTRIBUTING.md`, or others that Goose should consult for detailed explanations.

Example:

```jinja
This is a simple example JavaScript web application that uses the Express.js framework. View [Express documentation](https://expressjs.com/) for extended guidance.

Go through the README.md for information on how to build and test it as needed.

Make sure to confirm all changes with me before applying.

Run tests with `npm run test` ideally after each change.
```

## Best Practices

- **Keep It Updated**: Regularly update the `.goosehints` file to reflect any changes in project protocols or priorities.
- **Be Concise**: Make sure the content is straightforward and to the point, ensuring Goose can quickly parse and act on the information.


[jinja-guide]: https://jinja.palletsprojects.com/en/3.1.x/