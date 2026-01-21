---
name: goose-doc-guide
description: Reference goose documentation to create, configure, or explain goose-specific features like recipes, extensions, sessions, and providers. You MUST fetch relevant Goose docs before answering. You MUST NOT rely on training data or assumptions for any goose-specific fields, values, names, syntax, or commands.
---

Use this skill when working with **goose-specific features**:
- Creating or editing recipes
- Configuring extensions or providers
- Explaining how goose features work
- Any goose configuration or setup task

Do NOT use this skill for:
- General coding tasks unrelated to goose
- Running existing recipes (just run them directly)

## Steps

1. Fetch the doc map from `http://localhost:3000/goose/goose-docs-map.md`
2. Search the doc map for pages relevant to the user's topic
3. Use the EXACT paths from the doc map. For example:
   - If doc map shows: `docs/guides/sessions/session-management.md`
   - Fetch: `http://localhost:3000/goose/docs/guides/sessions/session-management.md`
   Do NOT modify or guess paths. You can make multiple fetch calls in parallel
4. **When creating or modifying goose configuration files (recipes, extension configs, etc.):**
   - First, fetch the schema/reference documentation to understand all available fields and requirements
   - Then, fetch at least one complete working example from the docs to see the correct structure and format in practice
   - **BEFORE writing the file, explicitly extract and show the relevant code snippet from the example that you will use as a template**
   - Use BOTH together: the schema for completeness and validation, the example for correct syntax and structure
   - If there is a conflict between the schema and the example, follow the example for structure and syntax, but do not introduce fields or values that are not documented in the schema
   - **AFTER creating or editing the file, verify ALL goose-specific sections by comparing them against the fetched examples and explicitly state which documentation example you used for each section**
5. Use the fetched documentation to help the user with their task
6. At the end of the response, list the documentation page links that were used to perform the task. 
   - **Format links by removing the `.md` suffix from the fetched URL**
   - Example: If you fetched `http://localhost:3000/goose/docs/guides/sessions/session-management.md`, 
     list it as `http://localhost:3000/goose/docs/guides/sessions/session-management`
   - Only include links that were actually fetched and referenced

## Strict Requirements
1. You MUST fetch relevant Goose documentation before providing any goose-specific information.

2. **Verification rule**:
   You may ONLY present goose-specific information that is explicitly stated in the fetched documentation.
   If you cannot identify the documentation page that supports a detail, you must not include it.
   
   When creating or modifying goose configuration files (recipes, extension configs, etc.), you must also:
   - Quote the relevant documentation snippet BEFORE writing each goose-specific section
   - After completing the file, review each goose-specific field and confirm it matches the fetched documentation

3. Do not add usage instructions, CLI examples, commands, or extra guidance unless:
   - the user explicitly asked for them, AND
   - the content appears exactly in the fetched documentation.