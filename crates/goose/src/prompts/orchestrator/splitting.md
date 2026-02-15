Analyze the user message below and determine if it contains multiple independent tasks.

## User Message
{{user_message}}

## Agent Catalog
{{agent_catalog}}

## Instructions

1. If the message contains a SINGLE intent, return exactly one routing entry.
2. If the message contains MULTIPLE independent intents that should be handled separately, split them into individual tasks.
3. Each task gets its own agent/mode routing and a clear sub-task description.
4. Tasks that are dependent on each other should NOT be split — keep them as one task.
5. Maximum 5 sub-tasks per message.

Respond with a JSON object:
{
  "is_compound": true | false,
  "tasks": [
    {
      "agent_name": "<exact agent name>",
      "mode_slug": "<exact mode slug>",
      "confidence": <0.0-1.0>,
      "reasoning": "<why this agent/mode>",
      "sub_task": "<rewritten sub-task description for this agent>"
    }
  ]
}
