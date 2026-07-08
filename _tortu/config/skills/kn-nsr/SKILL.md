---
name: kn-nsr
description: TRD New SME Review (NSR)
version: 1.2.0
author: Doug Daulton
metadata:
  hermes:
    tags: [TRD, NSR, review, SCR, reviewer, rubric]
---
# TRD – New SME Review (NSR)

NSR = **New SME Review**. Doug acts as SCR (reviewer): he dictates review
notes on ANOTHER SME's annotation set; you produce the reviewer report
sent to that SME. Allowed actions: **ADDITIONS, DELETIONS, EDITS, MERGES.**
Output: **Lightning Summary ONLY** – no Annotation Content. NEVER invent your
own methodology. Everything you need is inlined here and in SOUL. You have NO
file‑read tool – do not try to open the rubric or "the guidelines"; use the inline
rules and the worked example below.

## Clarify scope – ASK ONLY THESE, invent nothing
The ONLY things you may ask Doug before producing an NSR:
  (1) the expert's name, if he didn't give it;
  (2) resubmit‑for‑review vs. mark‑complete, if he didn't state it.
NEVER ask for a "timecode", "Context Reference ID", "reference ID", or any
metadata field – those do not exist in this workflow. Do not invent fields. If Doug's
notes say everything is fine / close it out / no errors, that is a NO‑ACTION review: skip
the item readback entirely and output the No‑Action template directly (only ask for the
expert name if missing).

## Greeting – exact form
The report's first line is the expert's name followed by a comma, nothing else: `Harrison,`
- never prefix it with "text", "salutation", or any label. Then a blank line, then the
body.

## READBACK – required before rendering the report
Before producing the final report, list back the items you are about to flag as a
numbered preview: `timecode – directive` (one line each), then ask Doug to confirm or correct.
Only render the formatted report after he confirms. This catches mis‑bound items and
dropped directives from long or multi‑pass dictation. Skip readback only if Doug says "no readback".

## Output – Lightning Summary (standard)
```text
[Expert Name],

Nice work. Below, I've identified a few items which need attention. [Once you've
made these changes, please resubmit this item for review. | Once you've
made these changes, please mark this item complete.]

Thanks – Doug

ADDITIONS
################
[timecode-start]: [textual-description]      # from Doug's review notes
TS-E: [timecode-end]
BB: {My description of the bounding box.}
ET: {My assigned error type.}

DELETIONS
################
[timecode-start]: [My notes explaining deletion]

EDITS
################
[timecode-start]: [textual-description]      # revised text per review notes
TS-S: {if changed – header timecode stays the ORIGINAL start}
TS-E: {if changed}
BB: {if changed}
ET: {if changed}

MERGES
################
[timecode-start]: [textual-description]      # earliest start; text covers all merged points
TS-S: {if changed}
TS-E: {if changed}
BB: {if changed}
ET: {if changed}
```

## Output – No‑Action Template
When a complete review yields ZERO actions across all four categories, do not emit empty
headers. Output exactly:
```text
[Expert Name],

All work reviewed, no errors identified. Therefore, no action is required. You can
mark this task complete.

Thanks – Doug
```

## TD rules for any annotation text you WRITE in the report
Most NSR entries are DIRECTIVES ("Change X to Y"), not full TDs. But if you write or
rewrite an actual textual description, it has THREE required parts, all mandatory:
1. `Unrealistic physics.` – literal opener (or `Unrealistic scene
   consistency.` only when Doug directs — including cross-camera /
   wrist-camera inconsistencies flagged as Other). For scene-consistency
   TDs, closer begins `This breaks scene consistency because…`.
2. Observation naming the specific part. May be one or more sentences
   for complex errors.
3. Closer with the physical reason: `This breaks physics because…` /
   `This is impossible because…` / `This is implausible because…`.
   Must describe why THIS SPECIFIC ERROR is physically impossible —
   never a copied generic physics sentence.

Sentence count: aim for ~3 for simple errors; more is correct when the
error is complex. Never exceed one short paragraph. Never pad with
repetition.

The observation alone is not a valid TD. See SOUL for gold‑standard exemplars.
Controlled vocabulary (banned: grabbers, claws, pinchers, clamps, teeth, pincers,
pilot, flange, indicated, numbered joints, timestamps, "robot arm"), camera‑perspective
left/right, no hedging, no "motivated cause/force", ET tag never inside the TD sentences.
Never write "robot" or "robot arm" in a TD — name the specific part.
Name specific objects ("the orange block") — never "the gripper" or "the object"
without a qualifier.

## POV / Wrist Camera Review Rules
Tasks may include a main camera view plus left and/or right wrist camera views.
When reviewing POV (multi-camera) annotations, apply these rules:

### [POV] MAJOR errors — flag these as DELETIONS or EDITS:
- Missing one or more required POV scene consistency annotations when
  camera views clearly contradict each other.
- Annotating a wrist camera view as the source of truth when the main
  camera view clearly shows the same implausibility and is not occluded,
  outside the frame, or impossible to verify. (Main camera is the
  authoritative view unless it is occluded/impossible to verify.)
- Failing to annotate the main camera view when it clearly shows an
  implausibility.
- Bounding box is placed in the wrong camera view.
- Failing to create separate annotations and separate bounding boxes
  for separate wrist camera inconsistencies when the left and right
  wrist camera views contradict the main camera view or each other.
- Textual description does not explain how the camera views are
  inconsistent when the annotation is for Other/Unrealistic scene
  consistency. (The TD must say which camera views differ and how.)
- Textual description incorrectly says an issue breaks physics when
  the annotation is only a scene consistency mismatch between otherwise
  plausible camera views. (Scene consistency mismatch → "Unrealistic
  scene consistency." opener + `This breaks scene consistency because…`
  closer — NOT `This breaks physics because…`.)

### [POV] MINOR errors — flag these as EDITS:
- Scene consistency annotation continues after the camera views have
  returned to sync (annotation end time too long).
- Scene consistency annotation ends too early while the inconsistency
  between camera views is still visible.
- Failing to identify which specific camera views are relevant in the
  POV textual description.
- Bounding box is not targeted to the specific error in a wrist camera
  view when it can be (bounding box too broad).

## Full Major/Minor Errors reference (for SME SCORE and review flagging)

### Major Errors
- Timestamp/chunk placed more than 250ms from error onset
- Bounding box not starting close to first frame or 2-3 frames into inconsistency
- Blatantly incorrect error category, or no error category selected
- Missing 1+ clear physics implausibility
- Bounding box/patch too large or too small
- "Other" selected when another category is clearly recognizable
- Operator's actions annotated as errors (operator is to be ignored)
- A video with errors contains a blank/no-violation submission
- Left/right references use operator perspective instead of camera perspective
- Combining different ET types in one annotation with overlapping bounding boxes
- Combining more than two of the same ET type in one annotation
- TD missing either the visual evidence summary OR the physics reason
- Physics sentence does not match what is actually happening
- Annotations not in complete sentences (bullet-point style)
- Weak TD: vague or generic physics reason, poor visual description
- Multiple annotations flagging plausible physics as errors
- [POV] any of the seven POV Major errors listed above

### Minor Errors
- Typos
- Annotation created for a task with no implausibilities
- Obviously low-confidence implausibility annotated without an FAQ post
- Annotation end time more than 250ms before or after implausibility ends
- Gripper terminology not consistent with thesaurus (e.g., "grabbers", "claws")
- Flagging objects that look unusual but are not implausible
- Reason for no violation not given in Reviewer notes on Blitz platform
- One annotation flagging plausible physics as an error
- [POV] any of the four POV Minor errors listed above

Note: Length differences (e.g., four vs. five sentences) should NOT
impact scoring unless sentences are clearly repetitive or unnecessary.

## Final Check – run this before sending, every time
1. All entries sorted by TS‑S ascending (Pass 2, Lightning Summary, every action section).
2. Blank line between every header, hash row, and entry.
3. Headers and hash counts exactly as the template (27 or 16).
4. Every dictated item is present – count them.
5. Timecodes exactly as dictated – no digit changes.
6. URSP used (not URS) for all robot-part ET references.
7. POV errors correctly categorized as Major or Minor per the rules above.
8. Nothing after the deliverable: no commentary, no recap.

## SME SCORE – internal block, ON REQUEST ONLY (paused 260607)
The SME scoring methodology is under revision and is NOT accurate yet. DO NOT
auto‑generate a score. Only produce this block if Doug explicitly asks ("score this" / "give me the SME score"). When he does: it is for Doug only, never sent to the expert;
score the expert's work is allowed, scoring your own output remains forbidden.

Scale: start at 5.0; subtract 1.0 per Major error, 0.5 per Minor error (floor 1.0).
Major/Minor definitions: use the Full Major/Minor Errors reference above.
Major deductions include all [POV] major errors.
Minor deductions include all [POV] minor errors.
Note: sentence count differences alone do not count as a deduction unless
sentences are clearly repetitive or unnecessary.

Format (blank line between every line):

SME SCORE (internal)

###########################

Score: [x.x] / 5

Majors: [count] – [one short line each, with timecode]

Minors: [count] – [one short line each, with timecode]

If the review found nothing: Score: 5 / 5, "No deductions."

## Action mapping – one item, ONE action
- Each reviewed item produces exactly ONE action entry. Never emit two entries (e.g., an EDIT and an ADDITION) for the same timecode unless Doug explicitly dictates two separate actions.
- If Doug dictates replacement wording, that is an EDIT – put his wording into the directive verbatim. It is never an ADDITION.
- NSR entries are DIRECTIVES to the expert ("Change X to Y"), not rewritten annotations. Include only the fields Doug commented on.
- If the expert's name was not given, ask for it (the one question) before producing the report.

## Worked Example (NSR) – real values, no placeholders, optional omitted
Doug dictates: "At 1.22 the error type is wrong – they used Phantom Motion but this is a robot part moving, so URSP. At 2.67 the bounding box is way too large, should just be on the gripper; text is fine. At 3.88 good catch but hedging language – 'appears to bend' should be 'bends'. Everything else looks fine. Expert is Keshav Prasad. Resubmit."

Correct output:
```text
Keshav Prasad,

Nice work. Below, I've identified a few items which need attention. Once you've made these changes, please resubmit this item for review.

Thanks – Doug

EDITS
################
1.22: ET: Change from Phantom Motion to URSP. Robot part movement is always URSP; Phantom Motion is reserved for scene objects.
2.67: BB: Tighten to cover the gripper only. Current box covers the entire scene.
3.88: TD: Remove hedging language. Change "appears to bend" to "bends."
```
