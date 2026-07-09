# Intent Compression

## The Pattern

As a user builds trust with an agent across a session, their prompts compress. Early in a session, prompts are verbose and self-describing. Later, they become terse directives that assume shared context. This is a signal of mastery, not simplicity - the user is operating at a higher level of delegation, not a lower level of cognition.

## Detection

Compression is detected when:
1. **Short prompt** (8 words or fewer) appears after the session is established (prompt index > 2)
2. **Known compression patterns** match: "proceed", "do it", "go", "yes", "continue", "ship it", "deploy", "merge", "lgtm", "next", "done", "send", "push", "run it", "build", "start", "finish"
3. **Very short directives** (4 words or fewer) appear late in a session (prompt index > 5)

## Impact on Scoring

When compression is detected:
- The prompt's DOK is elevated by +1 level (capped at 4)
- This produces the "Adjusted DOK" metric
- The difference between raw DOK and adjusted DOK is the "DOK lift"

## Compression as a Growth Signal

| Phase | Compression % | Interpretation |
|-------|---------------|----------------|
| Early (onboarding) | 0-3% | Barely present, every intent must be spelled out |
| Building | 1-3% | Not yet emerged as a pattern |
| Plateau | 3-5% | Beginning to appear as trust builds |
| Deepening | 8-15%+ | Trust made visible in the data |

Peak compression sessions correlate with highest DOK scores. Compression = short directives orchestrating complex workflows.

## Examples

| Prompt | Context | Surface DOK | Adjusted DOK |
|--------|---------|:-----------:|:------------:|
| "proceed" | After reviewing a multi-step plan | 1 | 2 |
| "check comments" | In a doc review session | 2 | 3 |
| "deploy" | After build + test cycle | 1 | 2 |
| "draft it" | After discussing stakeholder strategy | 2 | 3 |
| "yes" | Confirming a multi-tool research workflow | 1 | 2 |

## Key Insight

The cognitive complexity has moved from the prompt surface into the orchestration structure. A 2-word prompt that triggers a 500-word strategic response is not DOK 1 - it's compressed DOK 3.

**Source:** Observed during rp-why longitudinal analysis, Block Builder Fellowship (2026)
