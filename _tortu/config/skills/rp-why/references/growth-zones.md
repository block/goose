# Diagnostic Zones (ADT - Agentic Delegation Trust)

The TM x DOK matrix produces six diagnostic zones that reveal the health of the collaboration practice.

## Zone Definitions

| Zone | Description | Signal | Action |
|------|-------------|--------|--------|
| **Frontier** | TM and DOK matched and growing together | Operating at the productive edge | Document and share |
| **Growing** | Approaching a match between tool sophistication and cognitive depth | Building toward effective use | Keep pushing DOK 3+ work |
| **Expected** | Tool usage and cognitive depth appropriate for current level | Healthy starting position | Ask "why" before implementing |
| **Thinking Ahead** | Cognitive depth exceeds tool sophistication | Thinking at a higher level than tools support | Adopt more powerful orchestration |
| **Underutilizing** | Tool sophistication exceeds cognitive depth | Powerful tools for simple tasks | Deepen the questions being asked |
| **Overpowered** | Significant mismatch between tool complexity and task depth | Resources spent without proportional return | Redirect toward problems requiring analysis |

## The Matrix

```
              DOK 1         DOK 2          DOK 3           DOK 4
            (Recall)    (Application)  (Strategic)     (Extended)
           +----------+--------------+-------------+-------------+
Tier 5-6   | Over-    | Underutil-   |  Frontier   |  Frontier   |
(Symphony/ | powered  |   izing      |             |             |
 Virtuoso) |          |              |             |             |
           +----------+--------------+-------------+-------------+
Tier 3-4   | Over-    |  Expected    |   Growing   |  Frontier   |
(Ensemble/ | powered  |              |             |             |
 Chamber)  |          |              |             |             |
           +----------+--------------+-------------+-------------+
Tier 1-2   | Expected |   Growing    |  Thinking   |  Thinking   |
(Solo/     |          |              |   Ahead     |   Ahead     |
 Duet)     |          |              |             |             |
           +----------+--------------+-------------+-------------+
```

## Zone Calculation Logic

```
DOK >= 3.0 -> dok_band = 4
DOK >= 2.5 -> dok_band = 3
DOK >= 2.0 -> dok_band = 2
DOK <  2.0 -> dok_band = 1

TM Tier 5-6:
  dok_band >= 3 -> Frontier
  dok_band == 2 -> Underutilizing
  dok_band == 1 -> Overpowered

TM Tier 3-4:
  dok_band >= 4 -> Frontier
  dok_band == 3 -> Growing
  dok_band == 2 -> Expected
  dok_band == 1 -> Overpowered

TM Tier 1-2:
  dok_band >= 3 -> Thinking Ahead
  dok_band == 2 -> Growing
  dok_band == 1 -> Expected
```

**Source:** Dakota Fabro (2026), Three Dimensions Framework, Block Builder Fellowship
