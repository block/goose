---
name: kn-nrr
description: "TRD New Revision Round (NRR): implement an SCR reviewer's notes on Doug's prior annotation set field-level annotation content updates. Triggers: NRR, revision round, RNR (transposition of NRR), reviewer notes, implement these review notes, revise my annotations."
version: 2.2.0
author: Doug Daulton
metadata:
  hermes:
    tags: [TRD, NRR, revision, reviewer, annotation]
---
# TRD New Revision Round (NRR)
NRR=New Revision Round. Doug pastes a reviewer's notes on his prior annotation set; you produce the annotation content updates he moves into the web app. Allowed actions: ADDITIONS, DELETIONS, EDITs, MERGES. This skill produces ONLY the annotation content the Lightning Summary is a separate step (kn-summary), produced when Doug says "NRR complete." Do NOT produce a summary here.

Never invent methodology. Everything you need is in this skill and in SOUL
(the TD three-part rule + exemplars). You have NO file-read tool — do not
try to open the rubric or "the guidelines," and do not model your wording
on examples you cannot see. Use the inline exemplars below.

## Await state
If invoked with an await cue and no reviewer notes yet, reply with EXACTLY
`Awaiting reviewer notes.` and nothing else (see SOUL → AWAIT STATE) — no
plan, no rules recap, no reasoning. Produce the annotation updates only
after the notes arrive.

## Core rules
- Output ONLY the field(s) that changed per annotation. Never include unchanged annotations.
- EDIT/MERGE header timecode=the ORIGINAL start time; a changed start goes on a `TS-S:` line.
- DELETIONS rationale must be one of: invalid/subjective/covered elsewhere.
- MERGES: header=earliest annotation's start; text covers all merged points.
- Labels are EDITs (not "UPDATES") and ADDITIONS (not "NEW").
- End with FINAL ACTION: exactly `Marked as complete.` or `Resubmitted for new review.`, per the reviewer's directive.

## Number normalization (spoken->decimal timecode)
- ALWAYS apply Doug dictates by voice; transcription often leaves numbers as English words or mis-formats them. Convert EVERY timecode to decimal seconds before using or echoing it. Never leave English number words in output.
- "point seven five" -> 0.75; "two point two two" -> 2.22
- After "point", read digits individually: "one point oh four" -> 1.04; "five point oh six seven" -> 5.067; "zero point four seven eight" -> 0.478
- "two twenty-two" / "two twenty two" in timecode context -> 2.22 (NOT 222)
- "to the end" -> 5.067; spelled integers: "zero" -> 0 ... "five" -> 5
- Timecodes are ALWAYS within 0.00-5.067 (clip length). If a converted value falls outside that range, do NOT guess; flag it in the readback and ask Doug.
- Surface the converted decimals in the readback so Doug can catch any mis-conversion before the deliverable renders.

## READBACK required before rendering
Before producing the formatted deliverable, list back the items you captured as a numbered preview (`timecode, short description`, one line each) and ask Doug to confirm or correct. Render the final output only after he confirms. Catches mis-bound or dropped items from long/multi-pass dictation. Skip only if Doug says "no readback".

## Output (one ```text block, sections with items only)
```text
ADDITIONS
###########################
[timecode]: Unrealistic physics. [observation naming the part]. This breaks physics because [physical reason].
TS-E: [end time if extended, else omit line]

DELETIONS
###########################
[timecode]: [Original text to delete]
Rationale: [invalid/subjective/covered elsewhere]

EDITS
###########################
[timecode]: Unrealistic physics. [revised observation]. This breaks physics because [physical reason].
TS-E: [end time if extended, else omit line]

FINAL ACTION
###########################
Marked as complete.
```
- Top headers 27 hashes. Only include lines/fields that actually changed.
- Any new or revised TD is the full three-part form (opener · observation
  · closer) — see TD rules below and the worked example at the end.
- EDITS that change ONLY a non-text field (TS, BB, ET) state just that
  field as a directive (e.g. `1.88: ET: change Phantom Motion to URSP`) —
  no TD needed. Only write a full TD when the text itself changed.

## TD rules (any new or revised textual description) — ALL THREE PARTS REQUIRED
Every TD you write has THREE required parts on one line, in order:
1. `Unrealistic physics.` — the literal opener, verbatim. (Only
   `Unrealistic scene consistency.` instead, and only when Doug directs —
   including cross-camera / wrist-camera inconsistencies flagged as Other.
   For scene-consistency TDs, closer begins `This breaks scene consistency
   because…`, never `This breaks physics because…`.)
2. The observation — what is seen, naming the specific part. May be one
   or more sentences for complex errors.
3. The closer — the physical reason, beginning `This breaks physics
   because…` / `This is impossible because…` / `This is implausible
   because…`. Must describe why THIS SPECIFIC ERROR is impossible — never
   a copied generic physics sentence from another annotation.

Sentence count: aim for ~3 for simple errors; more is correct when the
error is complex. Never exceed one short paragraph. Never pad with
repetition. Four or five sentences is not wrong — missing either the
observation or the physics closer is.

The observation alone is NOT a valid TD. Dropping the `Unrealistic
physics.` opener, or replacing the closer with vague filler (\"breaks the
geometry of the system\", \"breaks the robot design\"), is WRONG. When Doug
hands you an observation, you still ADD the opener and WRITE the physical
reason — you are not just echoing his words.
- Observational tone, 11th-grade, definitive verbs. No subjective
  adjectives, no hedging (appears/seems/might/possibly/somehow), no
  "motivated cause/force", no engineering jargon.
- Never write "robot" or "robot arm" in a TD — name the specific part.
- No timestamps in TD text. Do not use the word "indicated" in TD text.
- Name specific objects ("the orange block", "the left gripper") — never
  "the gripper" or "the object" without a qualifier.
- ET tag never appears in the TD sentences — only on the `ET:` line.

## Vocabulary
- Arm: base, base joint, lower/upper shoulder link, lower/upper elbow link, forearm, wrist.
- Wrist sub-components: wrist head (top of wrist, black component);
  end effector connector (connects wrist to gripper, usually not visible —
  refer only if it is the locus of the error); wrist grip (rounded
  component above end effector connector, fixed); gripper camera (small
  white stick off end effector connector, rotates 360° only).
- Gripper: end effector (whole apparatus), fingers/jaws, fingertip, jaw
  tips, finger pads. One gripper per arm.
- Objects by attribute. Left/right=CAMERA perspective.
- BANNED: grabbers, claws, pinchers, clappers, clippers, clamps, teeth,
  pincers, pilot, flange, indicated, numbered joints, timestamps,
  "robot arm" (name the part).

## ET tags (ET: [line only]): URSP, PM, UGC, UI, UA, OCE, Other
(URSP=robot part errors — deformation/pose/speed/duplication/continuity;
PM=scene object no force; UGC=gripper contact fail; UI=interpenetration
objects only; UA=wrong speed/direction; OCE=object permanence objects only;
Other=Doug's call, scene consistency opener.)

URSP routing: arm-self-interpenetration → URSP not UI; arm–arm
interpenetration → URSP; gripper duplication → URSP; arm too fast → URSP
not UA. Exception: arm too fast AND object reacts at wrong speed → two
separate annotations: URSP + UA.

## Combining limit
Max TWO annotations of the same ET per combined annotation. Never combine
different ET types in one annotation if their bounding boxes overlap.

## Timecodes & edges
- Convert spoken numbers literally ("point seven five"=0.75; "to the end"=5.067). Echo exactly, never round/shift/drop. Do NOT change a timestamp unless the reviewer instructs it.
- Vague note, ask ONE question. Read/tool failure STOP and report.

## Output mechanics
- One ```text block in chat, real line breaks, never a file.
- [brackets]/{braces} are placeholders; substitute or omit; never print them literally.
- Nothing after the block, no commentary, scores, or recap.

## FINAL CHECK
1. Only changed fields/annotations present (no unchanged ones).
2. Entries sorted by TS-S ascending; headers 27 hashes.
3. FINAL ACTION line present and exact.
4. Timecodes exact; no banned vocabulary; no placeholder text.
5. Every TD that changed has ALL THREE parts: `Unrealistic physics.`
   opener + observation (specific part named, no "robot"/"robot arm",
   no "indicated", no timestamps) + `…because …` physics-reason closer
   matched to THIS error. Scene-consistency TDs use correct opener and
   `…scene consistency because…` closer. No vague or copied closers.
6. URSP used (not URS) for all robot-part ET tags.
7. Combining limit not violated.
8. One ```text block; nothing trailing.

## WORKED EXAMPLE (NRR) — real values, no placeholders
Reviewer notes Doug pastes:
"At 0.34 extend the end timestamp to 5.067, the issue persists. At 1.88
change the error type from Phantom Motion to URSP. At 2.45 tighten the
bounding box to just the wrist. Delete 3.10 — it's a consistent design
feature, not an anomaly. Add a new one around 4.33: the left elbow link
bends outward as the arm reaches left, URSP. Mark complete."

Correct output (note: only 4.33 and the TD-changing items get a full
three-part TD; field-only edits are bare directives):
```text
ADDITIONS
###########################
4.33: Unrealistic physics. The left elbow link bends outward as the arm reaches to the left. This breaks physics because the elbow link is a rigid structural component and should not bend during operation.
TS-E: 5.067
ET: URSP

DELETIONS
###########################
3.10: Consistent design feature of this arm throughout the clip — not an anomaly per reviewer.

EDITS
###########################
0.34: TS-E: 5.067
1.88: ET: change Phantom Motion to URSP
2.45: BB: tighten to the wrist area only

FINAL ACTION
###########################
Marked as complete.
```
Note what is ABSENT: no TD rewrite on 0.34/1.88/2.45 (only a field
changed), no placeholder brackets, no unchanged annotations.
