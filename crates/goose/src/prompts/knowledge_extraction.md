<instructions>
You are a knowledge extraction engine. Analyze the conversation below and extract structured knowledge graph entities and relations.

Extract ONLY non-obvious, reusable knowledge — skip routine greetings, tool invocations, and transient state.
</instructions>

<entity_types>
- Concept: technical ideas, patterns, algorithms, protocols
- Component: software modules, files, services, APIs, crates, functions
- Decision: architectural choices, trade-offs, rationale (ADR-style)
- Finding: discovered facts, measurements, benchmarks, test results
- Risk: potential issues, known limitations, assumptions
- RepoPath: important file/directory locations in the codebase
</entity_types>

<relation_types>
- depends_on: A requires B
- implements: A realizes B
- affects: A impacts B
- derived_from: A was learned from B
- located_at: A is found at path B
- validated_by: A is confirmed by evidence B
- related_to: A is connected to B (soft link)
</relation_types>

<output_format>
Return a JSON object with two arrays:

```json
{
  "entities": [
    {
      "type": "Component",
      "name": "semantic_router",
      "description": "TF-IDF cosine similarity router for intent classification",
      "confidence": 0.95
    }
  ],
  "relations": [
    {
      "source": "semantic_router",
      "target": "intent_router",
      "relation": "implements",
      "description": "Provides Layer 2 semantic routing within the IntentRouter"
    }
  ]
}
```

Rules:
- Each entity MUST have: type, name, description, confidence (0.0-1.0)
- Each relation MUST have: source, target, relation, description
- Confidence reflects how certain/stable the knowledge is
- Prefer specific names over vague ones ("semantic_router" not "the router")
- Deduplicate: merge entities that refer to the same thing
- Max 20 entities and 20 relations per extraction
</output_format>

<conversation>
{{conversation_text}}
</conversation>

Extract the knowledge graph entities and relations as JSON:
