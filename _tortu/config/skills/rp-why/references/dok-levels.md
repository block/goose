# Depth of Knowledge Levels (Webb, 1997)

Measures cognitive complexity of human prompts. Scored 1.0-4.0.

## The 4 Levels

### DOK 1: Recall & Reproduction
- **Prompts:** "What is...", "List...", "Define...", "Show me..."
- **Examples:** "What's the syntax for a Python list comprehension?"
- **Signal:** Asking for facts, syntax, definitions

### DOK 2: Application of Skills & Concepts
- **Prompts:** "How would you...", "Build...", "Fix...", "Implement..."
- **Examples:** "How would you implement this in TypeScript?"
- **Signal:** Applying learned skills to solve a specific problem

### DOK 3: Strategic Thinking
- **Prompts:** "Design...", "Analyze trade-offs...", "What if...", "Plan..."
- **Examples:** "Design a caching strategy considering our constraints"
- **Signal:** Reasoning across multiple concepts to plan, analyze, or design

### DOK 4: Extended Thinking
- **Prompts:** "Research and synthesize...", "Create a framework..."
- **Examples:** "Research authentication patterns across multiple sessions and synthesize recommendations"
- **Signal:** Creating something entirely new - frameworks, cross-disciplinary synthesis, novel artifacts

## Quick Assessment

| Signal | DOK Level |
|--------|-----------|
| Asking for facts, syntax, definitions | 1 |
| Asking how to apply, compare, explain | 2 |
| Asking to design, analyze, strategize | 3 |
| Multi-session investigation, framework creation | 4 |

## Adjusted DOK and Compression

Short prompts are not always simple prompts. As session trust builds, users compress their intent.

**Adjusted DOK** accounts for compression of intent - when short prompts carry complex meaning due to established context. A compressed "proceed" that triggers a multi-step architectural deployment is not DOK 1.

| Prompt | Surface DOK | Adjusted DOK | Why |
|--------|:-----------:|:------------:|-----|
| "proceed" | 1 | 2-3 | Triggers multi-step workflow in established context |
| "check comments" | 2 | 3-4 | Orchestrates 20+ comment review, research, strategic edits |
| "deploy" | 1 | 3 | Triggers build, test, deploy pipeline with established config |
| "draft it" | 2 | 3 | Generates context-aware communication requiring org understanding |

**Compression signals:**
- Short prompt (8 words or fewer) in an established session (prompt index > 2)
- Known compression patterns: "proceed", "do it", "go", "ship it", "deploy", etc.
- Late position in a long session (prompt index > 5) with very short directive (4 words or fewer)

When compression is detected, the prompt's DOK is elevated by +1 level (capped at 4).

## Key Metrics

- **DOK 3+4 %** - Proportion of strategic/extended thinking. The primary growth signal.
- **Compression %** - Short directives carrying complex intent. Trust made visible in data.
- **Floor** - Lowest DOK on routine days. When this rises, the baseline becomes unreproducible.

**Source:** Norman Webb (1997), Depth of Knowledge framework
