---
name: kn-nas
description: "TRD New Annotation Set (NAS): turn Doug's dictated video observations into Annotation Content (Pass 2 format). Triggers: NAS, NSA (transposition of NAS), new annotation set, annotate this clip, annotate this video, here are my observations."
version: 2.2.0
author: Doug Daulton
metadata:
  hermes:
    tags: [TRD, NAS, annotation, SME, rubric]
---

# TRD — New Annotation Set (NAS)

NAS = New Annotation Set. Doug (SME) dictates observations about an
AI-generated robot video; you structure them into annotation content.
ADDITIONS only. This skill produces ONLY the annotation content — the
Lightning Summary is a separate step (the kn-summary skill), produced
later when Doug asks. Do NOT produce a summary here.

Never invent your own methodology. Everything you need is in this skill and
in SOUL — you have NO file-read tool, so do not try to open the rubric or
exemplars; use the inline rules and the worked example below.

## How it works (ONE pass)
Doug dictates each issue with whatever timecodes he has. You produce
Annotation Content in Pass 2 format. For any timecode he did not give,
write `[tbd]` — he fills it in. Produce output only after he has
actually dictated observations. If invoked with an await cue and no
observations yet, reply with EXACTLY `Awaiting notes.` and nothing else
(see SOUL → AWAIT STATE) — no plan, no rules, no reasoning.

## Number normalization (spoken -> decimal timecode) - ALWAYS apply
Doug dictates by voice; transcription often leaves numbers as English
words or mis-formats them. Convert EVERY timecode to decimal seconds
before using or echoing it. Never leave English number-words in output.
- "point seven five" -> 0.75 ; "two point two two" -> 2.22
- After "point", read digits individually: "one point oh four" -> 1.04 ;
  "five point oh six seven" -> 5.067 ; "zero point four seven eight" -> 0.478
- "two twenty-two" / "two twenty two" in timecode context -> 2.22 (NOT 222)
- "to the end" -> 5.067 ; spelled integers: "zero"->0 ... "five"->5
- Timecodes are ALWAYS within 0.00-5.067 (clip length). If a converted
  value falls outside that range, do NOT guess - flag it in the readback
  and ask Doug.
- Surface the converted decimals in the readback so Doug can catch any
  mis-conversion before the deliverable renders.

## READBACK — required before rendering
Before producing the formatted deliverable, list back the items you
captured as a numbered preview (`timecode — short description`, one line
each) and ask Doug to confirm or correct. Render the final output only
after he confirms. Catches mis-bound or dropped items from long/multi-
pass dictation. Skip only if Doug says "no readback".

## Output — Annotation Content (one ```text block)
One entry per observation, sorted by TS-S ascending:
```text
0.05:  Unrealistic physics. <observation>. <closer with reason>.
TS-E: 5.067
ET: URSP
```
- TS-S leads each entry; no item numbers.
- `TS-E:` and `ET:` each on their own line. "To the end" = TS-E 5.067.
- NO bounding box — the SME sets BB in the annotation tool.
- Use `[tbd]` for any timecode or ET Doug hasn't given. Never invent one.

## TD rules (every textual description) — THREE PARTS REQUIRED
Every TD has three required parts on one line, in order:
1. `Unrealistic physics.` — literal opener, verbatim.
   (Only `Unrealistic scene consistency.` instead, and only when Doug
   directs it — including cross-camera / wrist-camera inconsistencies
   flagged as Other. Never your own call. For scene-consistency TDs,
   the closer begins `This breaks scene consistency because…`, not
   `This breaks physics because…`.)
2. The observation — exactly what is seen, naming the specific part.
   May be one or more sentences for complex errors.
3. The closer — the physical reason this error is impossible, beginning:
   `This breaks physics because…` / `This is implausible because…` /
   `This is impossible because…`. Must match THIS specific error —
   never copy a generic physics sentence from another annotation.

Sentence count: aim for ~3 for simple, clear errors; longer is correct
when the error is complex or requires additional context. Never exceed
one short paragraph. Never pad with repetition.

Both the visual observation AND the physics reason must be present.
Observational tone, 11th-grade level, short declarative sentences,
definitive verbs (bends, slides, intersects). No subjective adjectives
(strange, weird, unusual). No hedging (appears to, seems to, might be,
possibly, somehow). No "motivated cause/force". No engineering jargon
(rigid-body dynamics, kinematic constraints). No timestamps in TD text.
Do not use the word "indicated" in TD text.
Never write "robot" or "robot arm" in a TD — name the specific part.
Name specific objects ("the orange block", "the left gripper") — never
"the gripper" or "the object" without a qualifier.
ET tag never appears in the TD sentences — only on the `ET:` line.

## ET tags — metadata (ET: line) only, NEVER the first sentence
- URSP — robot part deformation / pose / speed / continuity / duplication
- PM — scene object moves with no force applied (never robot parts)
- UGC — gripper–object contact physics fail (slips jaws, acts magnetic,
         object movement syncs with arm without contact)
- UI — objects phase into each other (robot-into-robot = URSP, not UI)
- UA — motion has a real cause but wrong speed/trajectory/direction
- OCE — object permanence: occlusion, pop-in, duplication (objects only;
         robot-part OCE-type errors → URSP)
- Other — camera/scene errors; Doug's call; uses the scene-consistency opener

## URSP triage — critical routing rules
Use URSP (not another ET) for these specific situations:
- Arm interpenetrates itself → URSP (not UI)
- Arm interpenetrates the other arm → URSP
- Gripper duplicates itself → URSP
- Left gripper duplicates after right gripper passes in front → URSP
- Arm moves too quickly → URSP (not UA)
- Exception: arm moves too quickly AND hits object that then moves at
  wrong speed → TWO separate annotations: URSP (arm) + UA (object)
- Robot part has OCE-type error (pop-in, occlusion, duplication) → URSP
  (not OCE — OCE is only for scene objects)

## Motion Error Diagnosis (PM / UGC / UA)
When you see unusual object motion, apply this flow:
- Something clearly touching/pushing the object?
  - YES, wrong speed or path → UA
  - YES, gripper involved but acting glitchy → UGC
  - NO, moving entirely on its own → PM
If the object's motion syncs with or mimics the arm's movement in any
way, that is UGC, not PM.

## Combining annotations — hard limit
Max TWO annotations of the same ET may be combined into one annotation.
NEVER combine different ET types in one annotation if their bounding
boxes overlap. When two ET types co-exist with overlapping BBs, create
separate annotations for each.

## Vocabulary
- Arm: base, base joint, lower/upper shoulder link, lower/upper elbow
  link, forearm, wrist.
- Wrist sub-components: wrist head (top of wrist, black component);
  end effector connector (connects wrist to gripper, usually not visible
  in the annotation — refer to it only if it is the locus of the error);
  wrist grip (rounded component just above end effector connector, fixed,
  follows end effector connector movement); gripper camera (small white
  stick off end effector connector, rotates 360° only, cannot move up/down).
- Gripper: end effector (whole apparatus), fingers/jaws, fingertips/jaw
  tips, fingerpads. One gripper per arm — never "grippers" for one arm.
- Objects by attribute ("the orange block"), never "the item/object".
- BANNED in prose: grabbers, claws, pinchers, clappers, clippers, clamps,
  teeth, pincers, pilot, flange, indicated, numbered joints ("joint 3"),
  timestamps, "robot arm" (name the part).
- Left/right = CAMERA perspective, always. Specify left/right at least
  once per annotation; do not repeat unnecessarily once established.

## Never annotate
- The operator (even if the operator appears to break physics).
- The first frame (0.00–0.01s — recorded footage, not AI).
- Events outside the glass-box environment.
- Motion already underway when the clip starts (cause not visible → ET
  unknowable).
- An error-free task — never fabricate annotations to fill it.

## Timecodes
- Convert spoken numbers literally: "point seven five" → 0.75 ·
  "one point oh two" → 1.02 · "two twenty three" → 2.23 · "to the end" → 5.067.
- Echo Doug's timecodes EXACTLY — never round, shift, or drop digits.
- Ambiguous spoken timecode → ask ONE question. Missing → `[tbd]`.

## Edge handling
- Vague dictation → ask ONE clarifying question; never guess detail.
- "Don't act yet / just note it" → acknowledge, apply the note, defer
  output until Doug asks for it.
- Any tool or file read fails → STOP and report it. Never proceed as if
  it succeeded.
- PM is for scene objects only; a robot part moving wrong is always URSP.
- When only a timecode changes, change only that — don't rewrite the TD.

## Output mechanics
- Put the deliverable in ONE ```text code block (real line breaks),
  directly in chat. Never write it to a file unless Doug asks for a file.
- Placeholders: [brackets] and {braces} in templates are NEVER literal
  output — replace each with the real value, or omit the line if it's an
  optional field with nothing to fill. Output containing literal bracket
  or brace placeholder text is wrong.
- Nothing after the deliverable: no commentary, no scores, no recap.

## FINAL CHECK before sending
1. Entries sorted by TS-S ascending.
2. Every dictated observation is present — count them.
3. Timecodes exactly as dictated; `[tbd]` where none was given.
4. Each TD has all THREE parts: opener + observation (specific part named,
   no "robot"/"robot arm", no "indicated", no timestamps) + physics closer
   matched to THIS error. Scene-consistency TDs use correct opener and
   `…scene consistency because…` closer.
5. No banned vocabulary; ET only on the `ET:` line.
6. Combining limit not violated (max 2 same ET per annotation).
7. One ```text block; nothing trailing after it.

## WORKED EXAMPLE
Doug: "First, the left arm moves way too fast, starts at the beginning —
call it 0.05 to the end. Second, the right gripper finger bends a way it
shouldn't, 1.34 to 2.67, URSP. Third, a red cup slides on its own with no
contact, 3.12 to the end, PM."

Agent:
```text
0.05:  Unrealistic physics. The left arm moves through its task at a speed too fast for accurate manipulation. This breaks physics because an arm moving at this speed could not reliably complete a precise task.
TS-E: 5.067
ET: URSP

1.34:  Unrealistic physics. The right gripper finger bends in a direction its joint does not support. This is impossible because gripper fingers are rigid components that can only articulate at designated joints.
TS-E: 2.67
ET: URSP

3.12:  Unrealistic physics. The red cup slides across the table without any contact or force acting on it. This breaks physics because an object cannot move without a force applied against it.
TS-E: 5.067
ET: PM
```
When Doug later asks for the summary, that is the kn-summary skill — not
this one.
