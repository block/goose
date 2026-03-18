# The Nest — Specification and Setup Guide

> A persistent directory where goose organizes knowledge across sessions.

## Operating Model

The nest is a **goose workspace**. You start goose with the nest as your working directory. Hooks fire because goose finds `.goose/settings.json` in the working directory. The nest is not globally injected — it is a directory you work in.

```text
cd ~/.goose/nest && goose
```

That's it. Hooks fire. Context is injected. Knowledge compounds.

If you're working in a different directory (a repo, a project), the nest is not active. You can still read nest files manually, but hooks won't fire. This is intentional — the nest is opt-in per session via working directory.

## Directory Structure

```text
~/.goose/nest/
├── GUIDES/          # "How do I do X?" — verified procedures
├── RESEARCH/        # "What do we know about X?" — findings, analysis
├── PLANS/           # "What should we build?" — specs, proposals
├── WORK_LOGS/       # "What happened?" — session decision logs
├── REPOS/           # Cloned repos (working copies, not knowledge)
├── .scratch/        # Ephemeral intermediate files (deletable)
├── CATALOG.md       # Generated index (never hand-edit)
├── TAGS.md          # Canonical tag vocabulary
├── NEST.md          # Static conventions reference
├── build-catalog    # Catalog generator script
└── .goose/
    └── settings.json  # Hooks config
```

Flat within knowledge directories — no subdirectories. Every file fits in exactly one directory.

## File Format

Every `.md` file in GUIDES/, PLANS/, RESEARCH/, WORK_LOGS/ has YAML frontmatter:

```yaml
---
title: "Always Quoted Title"
tags: [lowercase-hyphenated, max-four-tags]
status: active
created: 2026-03-18
---
```

### Required Fields

| Field | Type | Rules |
|-------|------|-------|
| `title` | string | Always quoted (unquoted colons crash YAML parsers) |
| `tags` | list | 2-4 tags from TAGS.md. Lowercase, hyphenated, singular. |
| `status` | enum | `active` · `draft` · `stale` · `superseded` |
| `created` | date | YYYY-MM-DD |

### Optional Fields

| Field | Type | When |
|-------|------|------|
| `supersedes` | path | New file replaces old: `supersedes: RESEARCH/OLD_FILE.md` |
| `verified` | date | GUIDES only. Last confirmed working. |
| `sources` | list | GUIDES only. Paths to research files that fed the guide. |

### Status Lifecycle

`draft` → `active` → `stale` (outdated, no replacement) or `superseded` (replaced by newer doc)

### Filename Convention

`ALL_CAPS_WITH_UNDERSCORES.md`. WORK_LOGS are date-prefixed: `YYYYMMDD_HHMM_SLUG.md` (status always `active`, derive `created` from filename).

## CATALOG.md

Generated, never hand-edited. The `build-catalog` script scans all four knowledge directories, parses frontmatter, and produces:

- All Documents table (path, title, status, tags, modified date)
- Tag Index (tag → count → file list)

Run `./build-catalog` after adding or updating files. Idempotent — only writes CATALOG.md.

### Discovery Flow

1. Read CATALOG.md first (tag index + document table)
2. Grep CATALOG.md for topic keywords
3. Read matching files directly
4. Fall back to `rg` for keyword search across all files

## NEST.md

A **static** file in the nest root. Not generated. Contains an abridged version of the conventions (directory purposes, frontmatter format, discovery flow). This is what the SessionStart hook injects — not CATALOG.md alone, because CATALOG.md can grow large in a mature nest.

NEST.md is short (~30 lines) and stable. It tells the agent: here's what the nest is, here's how to use it, read CATALOG.md for what's in it.

## Hooks

Hooks live in `.goose/settings.json` inside the nest. They fire when goose starts with the nest as working directory.

**Prerequisite**: Project hooks must be enabled globally. The bootstrap script handles this automatically (see Bootstrap section).

`.goose/settings.json` in the nest:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cat NEST.md && echo '---' && cat CATALOG.md"
          }
        ]
      }
    ],
    "PostCompact": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cat NEST.md"
          }
        ]
      }
    ]
  }
}
```

| Hook | When | Injects |
|------|------|---------|
| `SessionStart` | Session begins | NEST.md (conventions) + CATALOG.md (full index) |
| `PostCompact` | After compaction | NEST.md only (keeps agent nest-aware without re-injecting full catalog) |

PostCompact fires on all compaction events (auto and manual). No matcher needed.

## Bootstrap

Copy-paste this entire block into a terminal. It creates a working nest.

**Prerequisites**: `uv` (for build-catalog). Install: `curl -LsSf https://astral.sh/uv/install.sh | sh`

```bash
bash << 'BOOTSTRAP'
set -euo pipefail

# ── Prerequisites ───────────────────────────────────────────────────
if ! command -v uv >/dev/null 2>&1; then
  echo "❌ uv is required but not installed."
  echo "   Install: curl -LsSf https://astral.sh/uv/install.sh | sh"
  exit 1
fi

NEST="$HOME/.goose/nest"

# ── Create directory structure ──────────────────────────────────────
mkdir -p "$NEST"/{GUIDES,RESEARCH,PLANS,WORK_LOGS,REPOS,.scratch,.goose}

# ── TAGS.md — seed vocabulary ───────────────────────────────────────
cat > "$NEST/TAGS.md" << 'EOF'
# Canonical Tag Vocabulary

| Tag | Description |
|-----|-------------|
| `architecture` | System architecture, design patterns |
| `bug-fix` | Bug fixes, debugging, root cause analysis |
| `deployment` | Deployment, CI/CD, infrastructure |
| `goose` | goose internals, extensions, configuration |
| `gooseclaw` | gooseclaw nest, orchestration, home screen |
| `hooks` | Agent lifecycle hooks, automation |
| `implementation` | Implementation details, code changes |
| `knowledge-management` | Knowledge organization, catalogs, tagging |
| `research` | Research findings, landscape analysis |
| `security` | Security patterns, sandboxing, access control |
| `testing` | Test patterns, coverage, CI |
EOF

# ── NEST.md — static conventions (injected by hooks) ───────────────
cat > "$NEST/NEST.md" << 'EOF'
# Nest

You are working in a persistent knowledge directory. Knowledge written here
persists across sessions.

## Directories

| Directory | Purpose |
|-----------|---------|
| GUIDES/ | Verified procedures — "How do I do X?" |
| RESEARCH/ | Findings and analysis — "What do we know about X?" |
| PLANS/ | Specs and proposals — "What should we build?" |
| WORK_LOGS/ | Session decision logs — "What happened and why?" |
| .scratch/ | Temporary working files (deletable) |

## Conventions

- Check CATALOG.md first before researching from scratch
- Files use YAML frontmatter: title, tags, status, created
- Filenames: ALL_CAPS_WITH_UNDERSCORES.md
- WORK_LOGS: date-prefixed YYYYMMDD_HHMM_SLUG.md
- Tags from TAGS.md, 2-4 per file
- Status: draft → active → stale or superseded
- Run ./build-catalog after adding files

## Writing to the Nest

- Intermediate findings → .scratch/
- Durable research → RESEARCH/
- Procedures → GUIDES/ (set verified: date when confirmed)
- Specs and proposals → PLANS/
- Session logs → WORK_LOGS/
EOF

# ── .goose/settings.json — hooks config ─────────────────────────────
cat > "$NEST/.goose/settings.json" << 'EOF'
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cat NEST.md && echo '---' && cat CATALOG.md"
          }
        ]
      }
    ],
    "PostCompact": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cat NEST.md"
          }
        ]
      }
    ]
  }
}
EOF

# ── Global hooks permission (idempotent) ────────────────────────────
GLOBAL_HOOKS="$HOME/.config/goose/hooks.json"
mkdir -p "$(dirname "$GLOBAL_HOOKS")"
if [ ! -f "$GLOBAL_HOOKS" ]; then
  echo '{"allow_project_hooks": true}' > "$GLOBAL_HOOKS"
  echo "✅ Created $GLOBAL_HOOKS"
else
  # Idempotently ensure allow_project_hooks is true using uv (already checked above)
  uv run --script - << 'PYFIX'
# /// script
# requires-python = ">=3.11"
# ///
import json, os
p = os.path.expanduser("~/.config/goose/hooks.json")
with open(p) as f: d = json.load(f)
if d.get("allow_project_hooks") is not True:
    d["allow_project_hooks"] = True
    with open(p, "w") as f: json.dump(d, f, indent=2)
    print(f"✅ Set allow_project_hooks: true in {p}")
else:
    print("✅ allow_project_hooks already enabled")
PYFIX
fi

# ── build-catalog script ────────────────────────────────────────────
cat > "$NEST/build-catalog" << 'BUILDCAT'
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pyyaml"]
# ///
"""build-catalog — Generate CATALOG.md from YAML frontmatter in knowledge files."""
import re, sys, yaml
from pathlib import Path
from datetime import datetime
from collections import defaultdict

ROOT = Path(__file__).parent
DIRS = ["GUIDES", "PLANS", "RESEARCH", "WORK_LOGS"]
TAGS_MD = ROOT / "TAGS.md"

def parse_tags():
    tags = set()
    if TAGS_MD.exists():
        for line in TAGS_MD.read_text().splitlines():
            m = re.match(r"^\|\s*`([^`]+)`\s*\|", line)
            if m: tags.add(m.group(1))
    return tags

def parse_fm(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0].strip() != "---": return None
    end = next((i for i, l in enumerate(lines[1:], 1) if l.strip() == "---"), None)
    if end is None: return None
    try: return yaml.safe_load("\n".join(lines[1:end]))
    except: return None

def main():
    canonical = parse_tags()
    records, tag_use = [], defaultdict(list)
    for d in DIRS:
        dp = ROOT / d
        if not dp.exists(): continue
        for f in sorted(dp.glob("*.md")):
            fm = parse_fm(f)
            rel = f.relative_to(ROOT).as_posix()
            mtime = datetime.fromtimestamp(f.stat().st_mtime).strftime("%Y-%m-%d")
            if fm and isinstance(fm, dict):
                title = fm.get("title", f.stem)
                status = fm.get("status", "?")
                tags = fm.get("tags", [])
                if not isinstance(tags, list): tags = []
                for t in tags: tag_use[t].append(rel)
                records.append((rel, title, status, tags, mtime))
                if canonical:
                    for t in tags:
                        if t not in canonical:
                            print(f"⚠️  {rel}: Tag '{t}' not in TAGS.md", file=sys.stderr)
            else:
                print(f"⚠️  {rel}: Missing or invalid frontmatter", file=sys.stderr)
                records.append((rel, f.stem, "?", [], mtime))

    out = [f"# Nest Catalog\n",
           f"*Auto-generated. Run `./build-catalog` to rebuild.*",
           f"*Last built: {datetime.now().isoformat(timespec='seconds')}*\n"]

    out.append(f"## All Documents ({len(records)} files)\n")
    out.append("| File | Title | Status | Tags | Modified |")
    out.append("|------|-------|--------|------|----------|")
    for rel, title, status, tags, mtime in records:
        t = ", ".join(f"`{x}`" for x in tags)
        out.append(f"| {rel} | {title} | {status} | {t} | {mtime} |")

    out.append(f"\n## Tag Index\n")
    out.append("| Tag | Count | Files |")
    out.append("|-----|-------|-------|")
    for tag, files in sorted(tag_use.items(), key=lambda x: (-len(x[1]), x[0])):
        shown = ", ".join(sorted(files)[:5])
        if len(files) > 5: shown += f" (+{len(files)-5} more)"
        out.append(f"| `{tag}` | {len(files)} | {shown} |")

    if not records:
        out.append("\n*Empty nest. Start working to build knowledge.*")

    (ROOT / "CATALOG.md").write_text("\n".join(out) + "\n")
    print(f"✅ CATALOG.md generated ({len(records)} files)", file=sys.stderr)

if __name__ == "__main__": main()
BUILDCAT
chmod +x "$NEST/build-catalog"

# ── Generate initial CATALOG.md ─────────────────────────────────────
cd "$NEST" && ./build-catalog

echo ""
echo "✅ Nest created at $NEST"
echo ""
echo "To use it:"
echo "  cd $NEST && goose"
echo ""
echo "The SessionStart hook will inject NEST.md + CATALOG.md automatically."
BOOTSTRAP
```

## Goosehints

Optional fallback for nest awareness without hooks. Add to `~/.config/goose/goosehints.md`:

```markdown
## Nest
Your persistent knowledge base is at ~/.goose/nest/. Check CATALOG.md before
researching from scratch. Write durable knowledge with YAML frontmatter.
Run ./build-catalog after adding files.
```

## What's NOT in This Spec

| Deferred | Why |
|----------|-----|
| Auto-run build-catalog on file write | Needs file watcher — too complex for v1 |
| Vector search / embeddings | Grep + CATALOG.md sufficient until ~500 files |
| Nest GUI in Electron home screen | Separate spec (gooseclaw v3.1) |
| Global injection into non-nest workspaces | The nest is a workspace, not a global overlay |
| Multi-nest / workspace switching | One nest. DirSwitcher exists for overrides. |

## Design Principles

1. **Files on disk.** Markdown with YAML frontmatter. Browsable in any editor, greppable with any tool.
2. **Push, don't pull.** Hooks inject context at session start. The nest announces itself.
3. **Generated index.** CATALOG.md is a build artifact. Source of truth is the files.
4. **Conventions, not enforcement.** Agents follow the structure because they're told to.
5. **Flat and simple.** No subdirectories. No complex schemas. Every file stands alone.
6. **The nest is a workspace.** Start goose in it. Hooks fire. Knowledge compounds.
