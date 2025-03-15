# Goose Bots

This document describes the JSON format for configuring specialized Goose bots that can be launched via deep links.

## Bot Configuration JSON Format

```json
{
  "id": "unique-bot-id",
  "name": "Bot Name",
  "description": "A short description of what this bot does",
  "instructions": "Detailed instructions that will be set as the system prompt for this bot",
  "activities": [
    "First suggested activity for this bot",
    "Second suggested activity for this bot",
    "Third suggested activity for this bot",
    "Fourth suggested activity for this bot",
    "Fifth suggested activity for this bot"
  ],
  "outputExample": "Example output format or URL to example"
}
```

### Fields Explanation

- `id`: A unique identifier for the bot (required)
- `name`: The display name of the bot (required)
- `description`: A brief description of the bot's purpose (required)
- `instructions`: The system prompt instructions for the bot (required)
- `activities`: An array of suggested activities that will replace the default SplashPills (required, recommended 5 items)
- `outputExample`: A short example of expected output format or a URL to an example (optional)

## Example Bot Configuration

```json
{
  "id": "code-reviewer",
  "name": "Code Reviewer",
  "description": "A specialized bot for reviewing code and suggesting improvements",
  "instructions": "You are a code review expert. Focus on identifying potential bugs, security issues, performance problems, and architectural concerns. Provide constructive feedback and suggest specific improvements with code examples when appropriate. Be thorough but concise, and prioritize the most important issues.",
  "activities": [
    "Review my JavaScript file for bugs",
    "Check my Python code for security issues",
    "Analyze performance of this React component",
    "Suggest improvements for my SQL query",
    "Help me refactor this function"
  ],
  "outputExample": "https://example.com/code-review-example"
}
```

## Implementation Details

The bot configuration is passed via a deep link, which sets the system prompt using the `/agent/prompt` endpoint and replaces the default SplashPills with the activities specified in the configuration.

The deep link format is:
`goose://bot?config=<base64-encoded-json>`

Where the JSON contains the bot configuration as described above.