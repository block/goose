# rp-why: Three Dimensions of AI Collaboration

> **> Goose Skill** - Measures DOK (cognitive complexity), TM (tool maturity), and ADT (agentic delegation trust) to reveal growth in human-AI collaboration practice.

A self-reflection framework that tracks three dimensions of your AI collaboration over time. See where you are, where you've been, and where the practice is heading.

## Installation

```bash
npx skills add https://github.com/block/agent-skills --skill rp-why
```

Make sure you have the built-in skills extension enabled in Goose.

## Quick Start

```
/rp-why init       # Generate baseline from history
/rp-why baseline   # Same as init
/rp-why current    # Analyze current session
/rp-why compare    # Compare session to baseline
/rp-why overall    # Full longitudinal analysis
```

**First time?** Start with `/rp-why init` to establish your baseline, then use `/rp-why current` at the end of sessions to track your practice.

## What It Measures

| Dimension | What | Scale |
|-----------|------|-------|
| **DOK** (Depth of Knowledge) | Cognitive complexity of your prompts | 1.0 - 4.0 |
| **TM** (Tool Maturity) | Intentional orchestration of AI tools | Tier 1-6 (Orchestra Model) |
| **ADT** (Agentic Delegation Trust) | Health of the collaboration practice | 6 diagnostic zones |

### Key Metrics in Every Report

- **DOK 3+4 %** - Proportion of strategic/extended thinking. The primary growth signal.
- **Compression %** - Short directives carrying complex intent. Trust made visible in data.
- **Floor** - Lowest DOK on routine days. When this rises, the baseline becomes unreproducible.
- **Adjusted DOK** - Accounts for compression of intent (short prompts that carry complex meaning due to established context).

## The Six Diagnostic Zones

The TM x DOK matrix produces zones that diagnose where your collaboration practice sits:

| Zone | What It Means | Action |
|------|---------------|--------|
| **Frontier** | TM and DOK matched and growing. Operating at the productive edge. | Document and share |
| **Growing** | Approaching a match. Building toward effective use. | Keep pushing DOK 3+ work |
| **Expected** | Appropriate match for current level. Healthy starting position. | Ask "why" before implementing |
| **Thinking Ahead** | Cognitive depth exceeds tools. Opportunity to adopt more powerful orchestration. | Explore sub-agents, multi-step delegation |
| **Underutilizing** | Tools exceed cognitive depth. Opportunity to deepen the questions. | Batch simple queries, ask bigger questions |
| **Overpowered** | Significant mismatch. Resources spent without proportional return. | Redirect toward problems requiring analysis |

## Orchestra Model (TM Tiers)

| Tier | Name | Description |
|------|------|-------------|
| 1 | Solo | Human works alone. AI reviews after. |
| 2 | Duet | Back-and-forth conversation. Human prompts, AI responds, human edits. |
| 3 | Ensemble | Human provides meaningful body of work. Evaluates holistically. |
| 4 | Chamber | Human delegates work streams. Sub-agents introduced. Orchestration required. |
| 5 | Symphony | Multiple AI interactions coordinated toward unified goal. Minimal intervention. |
| 6 | Virtuoso | Flow state. Human and AI synthesized. Optimal DOK, ADT, and TM. |

## Commands

### `/rp-why init` (alias: `/rp-why baseline`)

Establishes your starting point by analyzing all available session history.

**What you get:**
- Three Dimensions snapshot (DOK, TM tier, ADT zone)
- DOK distribution with bar chart
- DOK 3+4 % and Compression %
- Growth targets for the next level
- Baseline saved for future comparisons

### `/rp-why current`

Analyzes the active session. The quick-check mirror at the end of a work block.

**What you get:**
- Session snapshot with Three Dimensions
- DOK distribution for this session
- Peak moment (highest-DOK prompt highlighted)
- Contextual growth nudge based on your diagnostic zone
- Reflection question

### `/rp-why compare`

Delta report showing movement from your baseline to now.

**What you get:**
- Side-by-side Three Dimensions comparison with deltas
- DOK distribution shift (percentage point changes)
- Trajectory direction (improving / stable / declining)
- "What Shifted" narrative interpreting the changes

### `/rp-why overall`

Full longitudinal report. The complete growth picture across all sessions.

**What you get:**
- Current standings with all metrics
- Rolling average (first 10 vs. last 10 sessions)
- Phase analysis (auto-detected trajectory shifts)
- Peak DOK and floor tracking
- Full dataset DOK distribution
- Growth story narrative

## Example Output

```
==================================================================
                    rp-why . CURRENT SESSION
==================================================================

SESSION SNAPSHOT
------------------------------------------------------------------
Date:             Jun 18, 2026
Prompts:          31 classified

THREE DIMENSIONS
------------------------------------------------------------------
DOK (Adjusted)       2.45
TM  (Tool Maturity)  Tier 5 . Symphony
ADT (Delegation)     Frontier

DIAGNOSTIC ZONE: Frontier
------------------------------------------------------------------
TM and DOK matched and growing together. Operating at the
productive edge.

DOK DISTRIBUTION
------------------------------------------------------------------
DOK 1 (Recall      ):  #-------------------   6.5%
DOK 2 (Application ):  ##########----------  48.4%
DOK 3 (Strategic   ):  #######-------------  35.5%
DOK 4 (Extended    ):  ##------------------   9.7%

DOK 3+4:  45.2%     Compression:  12.9%

PEAK MOMENT
------------------------------------------------------------------
"Design the report format alignment between try-rpwhy and the..."
  DOK 3 . Strategic Thinking

GROWTH NUDGE
------------------------------------------------------------------
Strong session. The collaboration is matched and productive.
Consider extending one thread into a multi-session investigation.

  What complex challenge could benefit from sustained
   exploration across your next few sessions?
```

## Requirements

- [Goose](https://github.com/block/goose) AI agent
- Python 3.9+
- Goose sessions stored in the default session directory:
  - **macOS/Linux**: `~/.local/share/goose/sessions/`
  - **Windows**: `%LOCALAPPDATA%\goose\sessions\`

## Upgrading from v3

If you have an existing baseline from the Gas Town x DOK era (v3), it will be read and translated automatically:

- Gas Town stages map to Orchestra Tiers
- Quadrant names map to diagnostic zones
- "Learning Zone" becomes "Expected"
- Existing DOK scores are preserved as-is

No action needed. Run `/rp-why init` to generate a fresh v4 baseline, or let the skill auto-migrate your existing one.

## How It Works

1. **Reads your Goose session database** (SQLite, stored locally)
2. **Classifies each prompt** by DOK level using keyword pattern matching
3. **Detects compression** (short prompts in established context that carry complex intent)
4. **Calculates adjusted DOK** (+1 level for compressed prompts, capped at 4)
5. **Estimates TM tier** from session characteristics (tool usage, sub-agents, delegation patterns)
6. **Derives ADT zone** from the TM x DOK matrix
7. **Generates reports** with contextual nudges based on your diagnostic zone

All data stays local. Nothing is sent externally.

## Learn More

See [SKILL.md](./SKILL.md) for full documentation, including:
- Three Dimensions framework details
- Report format specifications
- Diagnostic zone matrix
- Growth nudge system
- Backward compatibility mapping
- Metrics reference

## Attribution

- **DOK Levels**: Norman Webb (1997). Criteria for alignment of expectations and assessments in mathematics and science education.
- **Orchestra Model (TM)**: Dakota Fabro (2026). Measuring Cognitive Complexity in Human-AI Collaboration.
- **Three Dimensions Framework**: Dakota Fabro (2026). rp-why longitudinal dataset, Block Builder Fellowship.
- **Gas Town Stages** (v1-v3 foundation): Steve Yegge, "Welcome to Gas Town" (January 2026).

---

## Changelog

### v4.0 (June 2026)

**Three Dimensions model.** The skill now tracks DOK, TM (Orchestra Tiers), and ADT (diagnostic zones) as three interconnected dimensions rather than the previous two-axis Gas Town x DOK model.

**What's new:**
- **Orchestra Model replaces Gas Town stages** - Six tiers (Solo through Virtuoso) describe how deliberately you coordinate AI tools. More intuitive than the 8-stage Gas Town scale and directly tied to observable session behavior.
- **Six diagnostic zones** replace four quadrants - Frontier, Growing, Expected, Thinking Ahead, Underutilizing, Overpowered. Derived from the TM x DOK matrix.
- **Compression tracking** - Detects when short prompts carry complex intent due to established context. Compression % appears in every report as a trust signal.
- **Adjusted DOK** - Primary DOK number now accounts for compression (+1 level for compressed prompts, capped at 4). Raw DOK still available.
- **DOK 3+4 %** - Aggregate metric showing proportion of strategic/extended thinking. The primary growth signal.
- **`/rp-why baseline` command** - Alias for `/rp-why init`. Both work identically.
- **`/rp-why overall` command** - New longitudinal report with phase analysis, rolling averages, floor tracking, peak DOK, and growth narrative.
- **Phase analysis** - Auto-detects trajectory shifts in the overall report.
- **Floor tracking** - Monitors the lowest DOK on routine days. A rising floor means the starting point is no longer reproducible.
- **Contextual growth nudges** - Nudges are now specific to your diagnostic zone rather than generic.
- **Backward compatibility** - v3 baselines (Gas Town x DOK, quadrant terminology) auto-migrate when loaded. No data loss.

**What's removed:**
- Gas Town stage numbers (1-8) no longer appear in reports. Orchestra Tiers (1-6) replace them.
- "Quadrant" terminology replaced by "diagnostic zone."
- Generic growth nudges replaced by zone-specific contextual nudges.

**Migration path:** Run `/rp-why init` to generate a fresh v4 baseline. Or do nothing - existing v3 baselines are auto-migrated on first read.

### v3.0 (February 2026)

- Quadrant visualization (Frontier, Growing, Thinking Ahead, Underutilizing)
- Growth nudges and reflection prompts
- Baseline comparison with `/rp-why compare`
- Target user profiles

### v2.x (January 2026)

- Integration matrix (Gas Town x DOK)
- Target profiles
- Baseline generation from session history

### v1.x (December 2025)

- Initial Gas Town stages
- Basic DOK tracking
