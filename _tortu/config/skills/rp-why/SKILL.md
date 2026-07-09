---
name: rp-why
description: Load when reflecting on AI collaboration quality or tracking growth over time. Analyzes session history to surface how deeply you think with AI tools and whether your orchestration sophistication matches your cognitive depth. Provides actionable nudges to push toward more strategic work.
author: dakotafabro
version: "4.0"
tags:
  - reflection
  - growth
  - ai-collaboration
  - self-assessment
  - productivity
  - learning
  - three-dimensions
---

# rp-why: Three Dimensions of AI Collaboration

## Overview

The **rp-why** skill is a self-reflection framework that helps AI practitioners measure and improve their collaboration practice. It tracks three dimensions:

- **DOK (Depth of Knowledge)** - Cognitive complexity of human prompts, scored 1.0-4.0 using Webb's framework. "Adjusted" accounts for compression of intent.
- **TM (Tool Maturity)** - Intentional orchestration of AI tools, tracked through the Orchestra Model (Tiers 1-6).
- **ADT (Agentic Delegation Trust)** - The relationship between tool sophistication and cognitive depth. Derived from the TM x DOK matrix.

The intersection of these dimensions produces **diagnostic zones** that reveal the health of the collaboration practice.

---

## How to Use This Skill

### Installation

```bash
npx skills add https://github.com/block/agent-skills --skill rp-why
```

### Available Commands

| Command | What It Does |
|---------|--------------|
| `/rp-why init` | Generate a baseline from your session history |
| `/rp-why baseline` | Same as init |
| `/rp-why current` | Analyze the current session |
| `/rp-why compare` | Compare current session to your baseline |
| `/rp-why overall` | Full longitudinal analysis across all sessions |
| `/rp-why token-spend` | Daily token spend breakdown across all sessions |

### Alternative: Natural Language

You can also ask naturally:

```
You: Analyze my AI collaboration patterns
You: What's my DOK distribution for this session?
You: How does this session compare to my baseline?
You: Give me the full rp-why longitudinal report
```

### When to Use

- **First time**: Run `/rp-why init` (or `/rp-why baseline`) to establish your starting point
- **End of session**: Run `/rp-why current` to reflect on your work
- **Weekly**: Run `/rp-why compare` to track progress against baseline
- **Monthly**: Run `/rp-why overall` for the full growth picture

---

## The Three Dimensions

### Dimension 1: DOK (Depth of Knowledge)

Measures the cognitive complexity of human prompts. Scored 1.0-4.0 using Webb's Depth of Knowledge framework.

| Level | Name | Description | Prompt Indicators |
|-------|------|-------------|-------------------|
| 1 | Recall & Reproduction | Simple factual prompts | "What is X?" "Show me the syntax for Y" |
| 2 | Application of Skills & Concepts | Applying learned skills to solve a problem | "Build this component" "Fix this error using pattern X" |
| 3 | Strategic Thinking | Reasoning across multiple concepts to plan, analyze, or design | "Design a system..." "Analyze trade-offs..." "What if..." |
| 4 | Extended Thinking | Creating something entirely new - frameworks, cross-disciplinary synthesis | "Research and synthesize..." "Create a framework..." |

**Adjusted DOK** accounts for compression of intent - when short prompts carry complex meaning due to established context. A compressed "proceed" that triggers a multi-step architectural deployment is not DOK 1.

**DOK 3+4 %** is the primary growth signal. It measures what proportion of your work operates at strategic or extended thinking levels.

**Compression %** tracks how often short directives carry complex intent. Compression emerges as trust deepens between practitioner and agent.

### Dimension 2: TM (Tool Maturity) - Orchestra Model

Measures intentional orchestration - how deliberately you coordinate AI tools, agents, and workflows. Only counts actions the user deliberately initiated.

| Tier | Name | Description |
|------|------|-------------|
| 1 | Solo | Human works alone. AI reviews after. |
| 2 | Duet | Back-and-forth conversation. Human prompts, AI responds, human edits. |
| 3 | Ensemble | Human provides meaningful body of work. Evaluates holistically. |
| 4 | Chamber | Human delegates work streams. Sub-agents introduced. Orchestration required. |
| 5 | Symphony | Multiple AI interactions coordinated toward unified goal. Minimal intervention. |
| 6 | Virtuoso | Flow state. Human and AI synthesized. Optimal DOK, ADT, and TM. |

### Dimension 3: ADT (Agentic Delegation Trust) - Diagnostic Zones

Measures the gap between Tool Complexity and Human Cognitive Depth. The TM x DOK matrix produces six zones:

| Zone | Description | Signal |
|------|-------------|--------|
| **Frontier** | TM and DOK matched and growing together | Operating at the productive edge |
| **Growing** | Approaching a match between tool sophistication and cognitive depth | Building toward effective use |
| **Expected** | Tool usage and cognitive depth appropriate for current level | Healthy starting position |
| **Thinking Ahead** | Cognitive depth exceeds tool sophistication | Opportunity to adopt more powerful orchestration |
| **Underutilizing** | Tool sophistication exceeds cognitive depth | Opportunity to deepen the questions being asked |
| **Overpowered** | Significant mismatch between tool complexity and task depth | Resources spent without proportional cognitive return |

---

## Diagnostic Zone Matrix

```
              DOK 1         DOK 2          DOK 3           DOK 4
            (Recall)    (Application)  (Strategic)     (Extended)
           +-----------+--------------+-------------+-------------+
Tier 5-6   | Overpowered| Underutil-  |  Frontier   |  Frontier   |
(Symphony/ |            |   izing     |             |             |
 Virtuoso) |            |             |             |             |
           +-----------+--------------+-------------+-------------+
Tier 3-4   | Overpowered|  Expected   |   Growing   |  Frontier   |
(Ensemble/ |            |             |             |             |
 Chamber)  |            |             |             |             |
           +-----------+--------------+-------------+-------------+
Tier 1-2   |  Expected  |   Growing   |  Thinking   |  Thinking   |
(Solo/     |            |             |   Ahead     |   Ahead     |
 Duet)     |            |             |             |             |
           +-----------+--------------+-------------+-------------+
```

---

## Report Formats

### `/rp-why init` or `/rp-why baseline`

Establishes the starting point. Analyzes all available session history.

**Report structure:**

```
+==================================================================+
|                    rp-why . BASELINE                             |
+==================================================================+

DATA SUMMARY
--------------------------------------------------------------------
Period:           [start] - [end] ([N] days)
Sessions:         [N]
Prompts:          [N] classified

THREE DIMENSIONS
--------------------------------------------------------------------
DOK (Depth of Knowledge)     [score]
TM  (Tool Maturity)          Tier [N] . [Name]
ADT (Delegation Trust)       [Zone]

DIAGNOSTIC ZONE: [Zone Name]
--------------------------------------------------------------------
[Zone description from the six-zone model]

DOK DISTRIBUTION
--------------------------------------------------------------------
DOK 1 (Recall):       [bar]  [%]
DOK 2 (Application):  [bar]  [%]
DOK 3 (Strategic):    [bar]  [%]
DOK 4 (Extended):     [bar]  [%]

DOK 3+4:  [%]
Compression:  [%]

GROWTH TARGETS
--------------------------------------------------------------------
DOK 3+4 target:       [%]
Compression target:   [%]
Next TM tier:         Tier [N] . [Name] ([what it means])

--------------------------------------------------------------------
Baseline saved to: ~/.config/goose/rp-why-baseline.json
Run /rp-why current after sessions to track progress.
```

### `/rp-why current`

Analyzes the active session. The quick-check mirror.

**Report structure:**

```
+==================================================================+
|                    rp-why . CURRENT SESSION                      |
+==================================================================+

SESSION SNAPSHOT
--------------------------------------------------------------------
Date:             [date]
Conversations:    [N]
Prompts:          [N] classified

THREE DIMENSIONS
--------------------------------------------------------------------
DOK (Adjusted)       [score]
TM  (Tool Maturity)  Tier [N] . [Name]
ADT (Delegation)     [Zone]

DIAGNOSTIC ZONE: [Zone Name]
--------------------------------------------------------------------
[Zone description]

DOK DISTRIBUTION
--------------------------------------------------------------------
DOK 1 (Recall):       [bar]  [%]
DOK 2 (Application):  [bar]  [%]
DOK 3 (Strategic):    [bar]  [%]
DOK 4 (Extended):     [bar]  [%]

DOK 3+4:  [%]     Compression:  [%]

PEAK MOMENT
--------------------------------------------------------------------
"[highest DOK prompt text, truncated]"  - DOK [N] . [Level Name]

GROWTH NUDGE
--------------------------------------------------------------------
[Contextual nudge based on diagnostic zone, not generic]

> [Reflection question tailored to current zone]
```

### `/rp-why compare`

Delta report. Shows movement from baseline to now.

**Report structure:**

```
+==================================================================+
|                    rp-why . COMPARE                              |
+==================================================================+

COMPARING: Today ([date]) vs. Baseline ([baseline period])

THREE DIMENSIONS                    Baseline        Today        Delta
--------------------------------------------------------------------
DOK (Adjusted)                      [score]         [score]    [+/-%]
TM  (Tool Maturity)                 Tier [N]        Tier [N]   [+/-N]
ADT (Delegation Trust)              [Zone]          [Zone]     [arrow]

DIAGNOSTIC ZONE: [Baseline Zone] -> [Current Zone]
--------------------------------------------------------------------

DOK DISTRIBUTION                    Baseline        Today        Delta
--------------------------------------------------------------------
DOK 1 (Recall)                      [%]             [%]       [+/-pp]
DOK 2 (Application)                 [%]             [%]       [+/-pp]
DOK 3 (Strategic)                   [%]             [%]       [+/-pp]
DOK 4 (Extended)                    [%]             [%]       [+/-pp]

DOK 3+4                             [%]             [%]       [+/-pp]
Compression                         [%]             [%]       [+/-pp or "emerged"]

TRAJECTORY
--------------------------------------------------------------------
Direction:  [arrow] [Improving/Stable/Declining]
Signal:     [1-2 sentence interpretation of what the delta means]

WHAT SHIFTED
--------------------------------------------------------------------
* [Bullet interpretations of the most meaningful changes]
* [Focus on what the numbers mean for the collaboration practice]
* [Connect to diagnostic zone movement]

--------------------------------------------------------------------
Run /rp-why overall for full longitudinal analysis.
```

### `/rp-why overall`

Full longitudinal report. The complete growth picture.

**Report structure:**

```
+==================================================================+
|                    rp-why . OVERALL                              |
+==================================================================+

FULL DATASET: [start] - [end] ([N] days)
Sessions: [N]  |  Prompts: [N]  |  Conversations: [N]

CURRENT STANDINGS
--------------------------------------------------------------------
DOK (Adjusted, full mean)    [score]
TM  (Tool Maturity)          Tier [N] . [Name]
ADT (Delegation Trust)       [Zone]

DOK 3+4:  [%]     Compression:  [%]     Floor:  [score]

ROLLING AVERAGE (Last 10 Sessions vs. First 10)
--------------------------------------------------------------------
                             First 10        Last 10          Delta
Adjusted DOK                 [score]         [score]        [+/-%]
DOK 3+4 %                   [%]             [%]           [+/-pp]
Compression %                [%]             [%]           [emerged/+/-pp]

PHASE ANALYSIS
--------------------------------------------------------------------
Phase         Dates              Sessions  DOK    DOK3+4  Comp   TM
-----         -----              --------  ---    ------  ----   --
[Auto-detected phases based on DOK trajectory shifts]

TRAJECTORY
--------------------------------------------------------------------
Peak DOK:     [score] ([date]  - [context])
Floor:        [score] ([interpretation])
Direction:    [arrow] [Narrative of the growth arc]

DOK DISTRIBUTION (Full Dataset)
--------------------------------------------------------------------
DOK 1 (Recall):       [bar]  [%]
DOK 2 (Application):  [bar]  [%]
DOK 3 (Strategic):    [bar]  [%]
DOK 4 (Extended):     [bar]  [%]

GROWTH STORY
--------------------------------------------------------------------
[2-3 sentence narrative interpreting the full arc. Not prescriptive.
Describes what the data reveals about how the collaboration practice
has evolved.]

--------------------------------------------------------------------
Data source: ~/.local/share/goose/sessions/sessions.db
Methodology: DOK keyword classification, compression detection
(short prompt + high response ratio + established context),
adjusted DOK (+1 level for compressed prompts, capped at 4).
```

---

## Backward Compatibility

### Reading Previous Baselines

If an existing baseline file uses the v3 format (Gas Town stages, quadrant terminology), the skill translates automatically:

| v3 Field | v4 Equivalent |
|----------|---------------|
| `estimated_stage: 5` | `tm_tier: 3` (Ensemble)  - Stage 5 maps to Tier 3 for new users |
| `quadrant: "Underutilizing"` | `adt_zone: "Underutilizing"` (zone name preserved) |
| `quadrant: "Frontier"` | `adt_zone: "Frontier"` |
| `quadrant: "Thinking Ahead"` | `adt_zone: "Thinking Ahead"` |
| `quadrant: "Learning Zone"` | `adt_zone: "Expected"` |
| `average_dok_score` | `dok_adjusted` (treated as raw if no compression data) |
| `growth_targets.dok_target` | `growth_targets.dok_3_4_pct` |

### Gas Town to Orchestra Mapping

For users familiar with the Gas Town stages:

| Gas Town Stage | Orchestra Tier | Name |
|----------------|----------------|------|
| 1-2 (Observer/Curious) | Tier 1 | Solo |
| 3 (Copilot) | Tier 2 | Duet |
| 4 (Chat IDE) | Tier 3 | Ensemble |
| 5 (CLI Agent) | Tier 4 | Chamber |
| 6 (Multi-Agent) | Tier 5 | Symphony |
| 7-8 (Agentic/Full) | Tier 6 | Virtuoso |

---

## Metrics Reference

| Metric | What It Measures | Range | Growth Signal |
|--------|-----------------|-------|---------------|
| DOK (Adjusted) | Cognitive complexity of prompts, accounting for compression | 1.0 - 4.0 | Rising mean |
| DOK 3+4 % | Proportion of strategic/extended thinking | 0 - 100% | Increasing |
| Compression % | Short directives carrying complex intent | 0 - 100% | Emerging, then growing |
| Floor | Lowest DOK on routine days | 1.0 - 4.0 | Rising (baseline becomes unreproducible) |
| TM Tier | Orchestra Model tier | 1 - 6 | Advancing |
| ADT Zone | Diagnostic zone from TM x DOK | 6 zones | Moving toward Frontier |

---

## Growth Nudges by Zone

### Frontier
- "Operating at the productive edge. Document what works for others."
- "The collaboration is matched. Look for opportunities to extend into new domains."

### Growing
- "Approaching a match. Keep pushing DOK 3+ work and the zone will shift."
- "Consider: what's one workflow you could delegate more fully?"

### Expected
- "Healthy starting position. Growth comes from asking 'why' before implementing."
- "Try framing one task as a design decision rather than an execution request."

### Thinking Ahead
- "Cognitive depth exceeds tool sophistication. Time to adopt more powerful orchestration."
- "Your thinking is ready for Tier [N+1]. Explore sub-agents or multi-step delegation."

### Underutilizing
- "Powerful tools deserve powerful questions. Before each prompt: can this be more strategic?"
- "Batch simple queries. Reserve the agent for work that requires reasoning."

### Overpowered
- "Significant mismatch. Consider whether this task needs an autonomous agent."
- "Opportunity: redirect this tool toward a problem that requires analysis or design."

---

## Attribution

- **Depth of Knowledge (DOK)**: Norman Webb (1997). Webb, N. L. Criteria for alignment of expectations and assessments in mathematics and science education.
- **Orchestra Model (TM)**: Dakota Fabro (2026). Measuring Cognitive Complexity in Human-AI Collaboration.
- **Three Dimensions Framework**: Dakota Fabro (2026). rp-why longitudinal dataset, Block Builder Fellowship.
- **Gas Town Stages** (v1-v3 foundation): Steve Yegge, "Welcome to Gas Town" (January 2026).

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 4.0 | 2026-06 | Three Dimensions model (DOK + TM + ADT), Orchestra Tiers, diagnostic zones, compression tracking, `/rp-why baseline` alias, `/rp-why overall` report, phase analysis, backward compatibility with v3 baselines |
| 3.0 | 2026-02 | Quadrant visualization, growth nudges, reflection prompts |
| 2.x | 2026-01 | Integration matrix, target profiles, baseline comparison |
| 1.x | 2025-12 | Initial Gas Town stages, basic DOK tracking |
