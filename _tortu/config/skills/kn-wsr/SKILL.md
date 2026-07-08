---
name: kn-wsr
description: "TRD Work Session Report (WSR): track a work session from WSS to WSE and produce the session summary. Triggers: WSR, WSS, WSE, work session starts, work session ends, session report."
version: 1.0.0
author: Doug Daulton
metadata:
  hermes:
    tags: [TRD, WSR, WSS, WSE, session, report]
---

# TRD — Work Session Report (WSR)

WSR = **Work Session Report**: the end-of-session summary of NAS / NRR /
NSR work completed. Authoritative rubric:
`commons/boundaries/guardrails/Kaminari_Rubric_v4_260604.md`. NEVER invent your own methodology.

## Workflow
- **WSS** (Work Session Starts): record the current timestamp, acknowledge
  it ("Work session started at YYYY-MM-DD HH:MM. Now tracking NAS, NRR,
  NSR.") and begin a running count of completed processes.
- **WSE** (Work Session Ends): record the timestamp, acknowledge
  ("Work session ended at YYYY-MM-DD HH:MM. Producing WSR...") then
  immediately output the Lightning Summary below.
- The WSS and WSE acknowledgments are EXACTLY the one specified line each —
  no plan, no rules recap, no reasoning, no preamble. Nothing else.

## Output — Lightning Summary (on WSE)
```
WORK COMPLETED
###########################
Session Start: [WSS timestamp]
Session End: [WSE timestamp]
Elapsed Time: [HH:MM]
New Annotation Sets: [x]
New Revision Rounds: [y]
New SME Reviews: [z]
```
- Header exactly 27 hashes. Counts cover only work between WSS and WSE.
- Use real clock time from the system, never estimated timestamps.

## Output rendering — REQUIRED
Wrap EACH deliverable (Annotation Content, Lightning Summary,
SME Score) in its own plain-text code block (```text fences) so line
breaks are preserved exactly and Doug gets a one-click copy button.
Inside the block, use real single line breaks — no extra blank lines
needed.

PLACEHOLDER SUBSTITUTION — CRITICAL: template tokens in [brackets] and
{braces} are placeholders, NEVER literal output. Replace each one
entirely with the real value from Doug's dictation. If an optional
field ({if provided...}) wasn't provided, OMIT that line completely.
If a required value is missing, ask ONE question. Output containing
literal [, ], {, } placeholder text or placeholder sentences is wrong.

## FINAL CHECK — run this before sending, every time
1. All entries sorted by TS-S ascending (Pass 2, Lightning Summary,
   every action section).
2. Blank line between every header, hash row, and entry.
3. Headers and hash counts exactly as the template (27 or 16).
4. Every dictated item is present — count them.
5. Timecodes exactly as dictated — no digit changes.
6. Nothing after the deliverable: no commentary, no recap.
