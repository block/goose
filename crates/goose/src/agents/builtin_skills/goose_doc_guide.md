---
name: goose-doc-guide
description: Reference goose documentation to create, configure, or explain goose-specific features like recipes, extensions, sessions, and providers. You MUST fetch relevant goose docs before answering. You MUST NOT rely on training data or assumptions for any goose-specific fields, values, names, syntax, or commands.
---

Use this skill when working with **goose-specific features**:
- Creating or editing recipes
- Configuring extensions or providers
- Explaining how goose features work
- Any goose configuration or setup task

Do NOT use this skill for:
- General coding tasks unrelated to goose
- Running existing recipes (just run them directly)

## Steps (COMPLETE ALL BEFORE RESPONDING)
1. **Fetch official docs**
   - Fetch the doc map from `https://block.github.io/goose/goose-docs-map.md`
   - Search the doc map for pages relevant to the user's topic and get the paths for these pages
   - Use the EXACT paths from the doc map. For example:
   - If doc map shows: `docs/guides/sessions/session-management.md`
   - Fetch: `https://block.github.io/goose/docs/guides/sessions/session-management.md`
   - Do NOT modify or guess paths. 
   - **ONLY fetch paths that are explicitly listed in the doc map - do not guess or infer URLs**
   - Make multiple fetch calls in parallel and save to temp files
   - Use the temp files for subsequent searches instead of re-fetching

2. **Create/modify content**
   - For goose configuration files:
      - Consult schema/field reference documentation first
      - **Search the fetched docs to extract the complete schema for each element you plan to use**
      - Extract example snippets to understand usage patterns
      - Create your configuration based on reference specs, following example patterns
      - **⚠️ STOP: Before showing the user, verify output content MUST match the schema and reference in goose the official documentation:**
         - [ ] Field names match exactly as shown in docs
         - [ ] Required fields/properties are present
         - [ ] Value formats match examples (YAML/JSON syntax, data types, etc.)
      - **If ANY verification fails, revise and repeat this step until ALL verifications pass**
      - **DO NOT present unverified output to the user**
   - Use only fetched docs for all goose-specific details

3. **MANDATORY VERIFICATION - DO THIS BEFORE STEP 4**
   Before writing your final answer:
   - [ ] You ONLY present goose-specific information that is explicitly stated in the fetched documentation. If you cannot identify the documentation page that supports a detail, you must NOT include it.
   - [ ] add instructions/commands if: (a) user explicitly asked for them, AND (b) they appear in the fetched docs
   - You MUST NOT rely on training data or assumptions for any goose-specific fields, values, names, syntax, or commands.
   - [ ] For EVERY goose-specific command in your answer: Search feteched files to verify it exists
   - [ ] For EVERY goose-specific field in your answer: Search feteched files to verify it exists  
   - [ ] For EVERY usage instruction in your answer: Search feteched files to verify it exists
   - [ ] If you cannot verify something: Remove it, unless the user explicitly asked for it (then state "I could not find documentation for [X]")
   
4. **Provide your answer and include a "Verification Completed" section showing which checks passed**
   - Which checks passed
   - How you verified each field exists in the documentation

5. **List documentation links**
   - Only include docs actually used
   - Remove `.md` suffix from URLs
   - Example: If you fetched `https://block.github.io/goose/docs/guides/sessions/session-management.md`, list it as `https://block.github.io/goose/docs/guides/sessions/session-management`