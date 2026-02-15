Given the user message below, select the best agent and mode from the catalog.

## User Message
{{user_message}}

## Agent Catalog
{{agent_catalog}}

Respond with a JSON object:
{
  "agent_name": "<exact agent name from catalog>",
  "mode_slug": "<exact mode slug from catalog>",
  "confidence": <0.0-1.0>,
  "reasoning": "<one sentence explaining why this agent/mode is best>"
}
