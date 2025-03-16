# Goose Bots

Goose Bots are specialized versions of Goose with custom instructions and suggested activities, launched via deep links.

## Creating a Bot Deep Link

1. Create a JSON object with your bot configuration:

```json
{
  "id": "my-bot",
  "name": "My Bot",
  "description": "Description of what my bot does",
  "instructions": "System prompt instructions for the bot",
  "activities": [
    "Suggested activity 1",
    "Suggested activity 2",
    "Suggested activity 3",
    "Suggested activity 4",
    "Suggested activity 5"
  ]
}
```

2. Convert this JSON to a base64 string:
   - Use an online tool like https://www.base64encode.org/
   - Or in JavaScript: `btoa(JSON.stringify(myBotConfig))`

3. Create a deep link with this format:
   ```
   goose://bot?config=YOUR_BASE64_STRING
   ```

4. Share this link - when opened, it will launch Goose with your custom bot configuration.

## Example

Here's a complete example for a SQL Assistant bot:

1. JSON Configuration:
```json
{
  "id": "sql-assistant",
  "name": "SQL Assistant",
  "description": "A specialized bot for SQL query help",
  "instructions": "You are an expert SQL assistant. Help users write efficient SQL queries and design databases.",
  "activities": [
    "Help me optimize this SQL query",
    "Design a database schema for a blog",
    "Explain SQL joins with examples",
    "Convert this query from MySQL to PostgreSQL",
    "Debug why this SQL query isn't working"
  ]
}
```

2. Base64 encoded (you can copy this):
```
eyJpZCI6InNxbC1hc3Npc3RhbnQiLCJuYW1lIjoiU1FMIEFzc2lzdGFudCIsImRlc2NyaXB0aW9uIjoiQSBzcGVjaWFsaXplZCBib3QgZm9yIFNRTCBxdWVyeSBoZWxwIiwiYWN0aXZpdGllcyI6WyJIZWxwIG1lIG9wdGltaXplIHRoaXMgU1FMIHF1ZXJ5IiwiRGVzaWduIGEgZGF0YWJhc2Ugc2NoZW1hIGZvciBhIGJsb2ciLCJFeHBsYWluIFNRTCBqb2lucyB3aXRoIGV4YW1wbGVzIiwiQ29udmVydCB0aGlzIHF1ZXJ5IGZyb20gTXlTUUwgdG8gUG9zdGdyZVNRTCIsIkRlYnVnIHdoeSB0aGlzIFNRTCBxdWVyeSBpc24ndCB3b3JraW5nIl0sImluc3RydWN0aW9ucyI6IllvdSBhcmUgYW4gZXhwZXJ0IFNRTCBHC3Npc3RhbnQuIEhlbHAgdXNlcnMgd3JpdGUgZWZmaWNpZW50IFNRTCBXDWVYATWVZMBHBM4GZGVZAWDUIGRHDHRHYMFZZXMUIN
```

3. Deep link:
```
goose://bot?config=eyJpZCI6InNxbC1hc3Npc3RhbnQiLCJuYW1lIjoiU1FMIEFzc2lzdGFudCIsImRlc2NyaXB0aW9uIjoiQSBzcGVjaWFsaXplZCBib3QgZm9yIFNRTCBxdWVyeSBoZWxwIiwiYWN0aXZpdGllcyI6WyJIZWxwIG1lIG9wdGltaXplIHRoaXMgU1FMIHF1ZXJ5IiwiRGVzaWduIGEgZGF0YWJhc2Ugc2NoZW1hIGZvciBhIGJsb2ciLCJFeHBsYWluIFNRTCBqb2lucyB3aXRoIGV4YW1wbGVzIiwiQ29udmVydCB0aGlzIHF1ZXJ5IGZyb20gTXlTUUwgdG8gUG9zdGdyZVNRTCIsIkRlYnVnIHdoeSB0aGlzIFNRTCBxdWVyeSBpc24ndCB3b3JraW5nIl0sImluc3RydWN0aW9ucyI6IllvdSBhcmUgYW4gZXhwZXJ0IFNRTCBHC3Npc3RhbnQuIEhlbHAgdXNlcnMgd3JpdGUgZWZmaWNpZW50IFNRTCBXDWVYATWVZMBHBM4GZGVZAWDUIGRHDHRHYMFZZXMUIN
```

## Testing

You can test bot functionality by selecting "Launch SQL Bot (Demo)" from the File menu or tray menu. This launches a pre-configured SQL Assistant bot that demonstrates:

1. Custom system prompt for SQL assistance
2. SQL-specific suggested activities
3. A notification when the bot is successfully configured