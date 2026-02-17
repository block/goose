#!/usr/bin/env python3
"""
Reload the Knowledge Graph from disk into the MCP memory server.

Usage:
    python3 docs/kg/reload.py [--graph docs/kg/graph.json]

This script reads the exported graph.json and replays all entities and relations
into the Knowledge Graph Memory MCP server via stdin/stdout JSON-RPC.

For use with goose, you can also reload manually by asking:
    "Load the knowledge graph from docs/kg/graph.json"

The reload process:
1. Read graph.json (contains entities[] and relations[])
2. For each entity: call create_entities with name, entityType, observations
3. For each relation: call create_relations with from, to, relationType

Note: The MCP memory server is idempotent for entities (upsert by name).
Relations are additive — duplicates may occur if reloaded multiple times.
"""

import json
import sys
import argparse


def load_graph(path: str) -> dict:
    """Load the graph from a JSON file (supports .gz)."""
    import gzip

    if path.endswith(".gz"):
        with gzip.open(path, "rt", encoding="utf-8") as f:
            return json.load(f)
    with open(path) as f:
        return json.load(f)


def print_stats(data: dict):
    """Print graph statistics."""
    entities = data.get("entities", [])
    relations = data.get("relations", [])
    print(f"Entities: {len(entities)}")
    print(f"Relations: {len(relations)}")

    types = {}
    for e in entities:
        t = e.get("entityType", "Unknown")
        types[t] = types.get(t, 0) + 1
    print("\nEntity types:")
    for t, c in sorted(types.items(), key=lambda x: -x[1]):
        print(f"  {t}: {c}")

    rel_types = {}
    for r in relations:
        t = r.get("relationType", "unknown")
        rel_types[t] = rel_types.get(t, 0) + 1
    print("\nRelation types:")
    for t, c in sorted(rel_types.items(), key=lambda x: -x[1]):
        print(f"  {t}: {c}")


def generate_reload_instructions(data: dict, batch_size: int = 20) -> list[dict]:
    """
    Generate MCP tool call payloads for reloading the graph.
    Returns a list of {tool, params} dicts.
    """
    entities = data.get("entities", [])
    relations = data.get("relations", [])
    calls = []

    # Batch entities
    for i in range(0, len(entities), batch_size):
        batch = entities[i : i + batch_size]
        calls.append(
            {
                "tool": "knowledgegraphmemory__create_entities",
                "params": {
                    "entities": [
                        {
                            "name": e["name"],
                            "entityType": e["entityType"],
                            "observations": e.get("observations", []),
                        }
                        for e in batch
                    ]
                },
            }
        )

    # Batch relations
    for i in range(0, len(relations), batch_size):
        batch = relations[i : i + batch_size]
        calls.append(
            {
                "tool": "knowledgegraphmemory__create_relations",
                "params": {
                    "relations": [
                        {
                            "from": r["from"],
                            "to": r["to"],
                            "relationType": r["relationType"],
                        }
                        for r in batch
                    ]
                },
            }
        )

    return calls


def main():
    parser = argparse.ArgumentParser(description="Reload Knowledge Graph from disk")
    parser.add_argument(
        "--graph", default="docs/kg/graph.json", help="Path to graph.json"
    )
    parser.add_argument(
        "--stats-only", action="store_true", help="Only print statistics"
    )
    parser.add_argument(
        "--emit-calls",
        action="store_true",
        help="Emit MCP tool calls as JSON (for programmatic reload)",
    )
    parser.add_argument(
        "--batch-size", type=int, default=20, help="Entities/relations per batch"
    )
    args = parser.parse_args()

    data = load_graph(args.graph)

    if args.emit_calls:
        calls = generate_reload_instructions(data, args.batch_size)
        # Stats go to stderr, JSON to stdout
        print(f"# {len(calls)} tool calls generated", file=sys.stderr)
        json.dump(calls, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")
        return

    print_stats(data)

    if args.stats_only:
        return

    # Interactive mode — print instructions for goose
    print("\n" + "=" * 60)
    print("TO RELOAD THIS GRAPH INTO GOOSE:")
    print("=" * 60)
    print()
    print("Ask goose:")
    print(
        '  "Reload the knowledge graph from docs/kg/graph.json '
        "using create_entities and create_relations in batches"
        ' of 20"'
    )
    print()
    print("Or use --emit-calls to generate programmatic tool calls:")
    print(f"  python3 {args.graph.rsplit('/', 1)[0]}/reload.py --emit-calls")


if __name__ == "__main__":
    main()
