---
name: kn-summary
description: "TRD Lightning Summary: produce the Lightning Summary for the NAS or NRR work just completed in this session. A follow-on step, requested after the annotation content is written. Triggers: lightning summary, produce the summary, give me the summary, summarize this, NRR complete, session complete."
version: 1.0.0
author: Doug Daulton
metadata:
  hermes:
    tags: [TRD, lightning summary, NAS, NRR, summary]
---

# TRD — Lightning Summary

This is the FOLLOW-ON step. Doug has already produced annotation content
(via kn-nas or kn-nrr) earlier in this session; now he wants the
Lightning Summary that goes into the system. Summarize the annotation
content already produced in THIS session — do not re-derive or re-format
the annotations themselves, and do not invent new ones.

Pick the mode from what was just done:
- The session produced NAS annotation content → use **NAS mode**.
- The session produced NRR annotation content → use **NRR mode**.
- (NSR has no separate summary step — its report is produced whole by
  kn-nsr. If asked to "summarize" an NSR, point Doug to kn-nsr.)
If you cannot tell which mode, ask Doug one question.

## NAS mode — ANNOTATIONS MADE
One line per annotation, sorted by TS-S ascending:
```text
ANNOTATIONS MADE
###########################
0.05: Added new annotation describing the left arm moving at unrealistically fast speed
1.34: Added new annotation describing the right gripper finger bending beyond its joint range
3.12: Added new annotation describing the red cup sliding without applied force
```
- Header exactly 27 hashes. Plain-English one-liner per item — not the
  full TD. ADDITIONS only (NAS never deletes/edits/merges).

## NRR mode — ACTIONS TAKEN
Nested: top header 27 hashes, each action sub-header 16 hashes. Include
ONLY the action sections that have items. End with FINAL ACTION.
```text
ACTIONS TAKEN
###########################

ADDITIONS
################
4.33: Added new annotation describing left elbow link bending outward

DELETIONS
################
3.10: Removed annotation; consistent design feature, not an anomaly

EDITS
################
0.34: End timestamp extended to 5.067
1.88: Error type changed from Phantom Motion to URS

FINAL ACTION
###########################
Marked as complete.
```
- FINAL ACTION is exactly `Marked as complete.` or `Resubmitted for new
  review.` — per the reviewer's directive in the NRR.

## Rules (both modes)
- Output the code block DIRECTLY — no preamble, no plan, no reasoning
  narration before it, nothing after it.
- One ```text code block, directly in chat, real line breaks. Never a file.
- Sort every section by TS-S ascending.
- Omit any action section with no items. Never emit an empty header.
- No [bracket]/{brace} placeholder text in the output.
- Nothing after the block — no commentary or recap.
