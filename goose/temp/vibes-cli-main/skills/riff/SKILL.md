---
name: riff
description: Generate 3-10 app variations in parallel for comparing ideas. Use when user says "explore options", "give me variations", "riff on this", "brainstorm approaches", or wants to see multiple interpretations of a concept.
allowed-tools: Read, Write, Bash, Task, Glob, AskUserQuestion
---

**Display this ASCII art immediately when starting:**

```
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓███████▓▒░░▒▓████████▓▒░░▒▓███████▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓█▓▒▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓█▓▒▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░░▒▓██████▓▒░  ░▒▓██████▓▒░
  ░▒▓█▓▓█▓▒░ ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░
  ░▒▓█▓▓█▓▒░ ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░
   ░▒▓██▓▒░  ░▒▓█▓▒░▒▓███████▓▒░░▒▓████████▓▒░▒▓███████▓▒░
```

# Vibes Riff Generator

Generate multiple app variations in parallel. Each riff is a different INTERPRETATION - different ideas, not just styling.

## Workflow

### Step 1: Gather ALL Requirements Upfront

**Use AskUserQuestion to collect all config at once.**

```
Question 1: "What kind of app do you want to explore? (broad/loose is fine)"
Header: "Theme"
Options: User enters via "Other"

Question 2: "Describe the visual style - colors, mood, aesthetic"
Header: "Visual"
Options: ["Warm sunset tones", "Clean minimal white", "Neon cyberpunk", "Other (describe)"]

Question 3: "How many variations should I generate?"
Header: "Count"
Options: ["3 (recommended)", "5", "7", "10"]
```

After receiving all answers, **proceed immediately to Step 2** - no more questions.

**Note:** If the `frontend-design` skill is available, use it for enhanced visual design quality.

### Step 2: Create Riff Directories
```bash
mkdir -p riff-1 riff-2 riff-3 ...
```

### Step 3: Generate Riffs in Parallel

**Use the bundled script to generate riffs in parallel.** Each script instance calls `claude -p` (uses subscription tokens) and writes directly to disk.

Generate riffs in parallel based on user's count:
```bash
# For each N from 1 to ${count}:
node "${CLAUDE_PLUGIN_ROOT}/scripts/generate-riff.js" "${prompt}" N riff-N/app.jsx "${visual}" &

# Then wait for all
wait
echo "All ${count} riffs generated!"
```

Example for count=3:
```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/generate-riff.js" "productivity apps" 1 riff-1/app.jsx "warm sunset oranges and soft creams" &
node "${CLAUDE_PLUGIN_ROOT}/scripts/generate-riff.js" "productivity apps" 2 riff-2/app.jsx "warm sunset oranges and soft creams" &
node "${CLAUDE_PLUGIN_ROOT}/scripts/generate-riff.js" "productivity apps" 3 riff-3/app.jsx "warm sunset oranges and soft creams" &
wait
```

**Why this works:**
- Each script calls `claude -p "..."` → uses subscription tokens
- Script writes directly to disk → no tokens flow through main agent
- Background processes (`&`) run in parallel → true concurrency
- Main agent only sees "✓ riff-N/app.jsx" output → minimal tokens

### Step 4: Assemble HTML

Convert each app.jsx to a complete index.html:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/assemble-all.js" riff-1 riff-2 riff-3 ...
```

### Step 5: Evaluate & Rank

Read the **pitch.md** files (NOT the full code) for fast evaluation:

```bash
# Read pitch files - contains reasoning about theme, colors, functionality
cat riff-*/pitch.md

# Also read BUSINESS comments for app names (just first 10 lines of each)
head -10 riff-*/app.jsx
```

**pitch.md** contains the model's reasoning about theme, colors, and design choices.
**BUSINESS comment** (top of app.jsx) contains: name, pitch, customer, revenue.

Then create RANKINGS.md using this format:

```markdown
# Riff Rankings: [Theme]

## Summary

| Rank | App Name | Score | Best For |
|------|----------|-------|----------|
| 1 | [Name] | XX/50 | [one-liner] |
| 2 | [Name] | XX/50 | [one-liner] |
| ... | ... | ... | ... |

## Detailed Scores

### #1: [App Name] (riff-N)
| Criterion | Score | Notes |
|-----------|-------|-------|
| Originality | X/10 | [Why unique or derivative] |
| Market Potential | X/10 | [Target audience size, demand] |
| Feasibility | X/10 | [Technical complexity, time to build] |
| Monetization | X/10 | [Revenue model viability] |
| Wow Factor | X/10 | [First impression, engagement] |
| **Total** | **XX/50** | |

[Repeat for each riff]

## Recommendations

- **Best for solo founder:** [riff-N] - [reason]
- **Fastest to ship:** [riff-N] - [reason]
- **Most innovative:** [riff-N] - [reason]
- **Best monetization:** [riff-N] - [reason]
```

**Scoring Guidelines:**
- 1-3: Poor - significant issues
- 4-5: Below average - notable weaknesses
- 6-7: Average - meets expectations
- 8-9: Good - stands out positively
- 10: Excellent - exceptional in this criterion

### Step 6: Generate Gallery

Create index.html gallery page with:
- Dark theme (#0a0a0f background)
- Glass-morphism cards with purple/cyan accents
- Each card: rank badge, app name, pitch, score bar, "Launch →" link
- Responsive grid layout
- Self-contained with inline styles

### Step 7: Present Results

```
Generated ${count} riffs for "${prompt}":
#1: riff-X - App Name (XX/50)
#2: riff-Y - App Name (XX/50)
...

Open index.html for gallery, or browse riff-1/, riff-2/, etc.
```

## Plugin Directory

The plugin root is available via `${CLAUDE_PLUGIN_ROOT}` which Claude Code resolves at runtime.

---

## What's Next?

After presenting rankings, guide the user with AskUserQuestion:

```
Question: "You have ${count} variations ranked. What would you like to do?"
Header: "Next"
Options:
- Label: "Develop the #1 ranked app"
  Description: "Take the top-ranked variation and continue building it. I'll copy riff-1/app.jsx to your working directory so you can iterate with the full vibes feature set - add functionality, refine the design, make it yours."

- Label: "Let me pick a different one"
  Description: "The rankings are just my analysis - you might see something special in another variation. Tell me which riff number speaks to you and I'll set it up for development."

- Label: "Turn winner into SaaS (/sell)"
  Description: "Ready to monetize the best one? Sell adds Clerk authentication with passkeys, subscription billing, and multi-tenant database isolation. Each customer gets their own subdomain with their own data."

- Label: "Deploy a riff as demo (/exe)"
  Description: "Want to share one quickly? Deploy puts any variation live at yourapp.exe.xyz within minutes. Great for getting feedback before committing to further development."

- Label: "I'm done for now"
  Description: "Wrap up this session. All riffs are saved in riff-1/, riff-2/, etc. - browse them locally or come back later to continue."
```

**After user responds:**
- "Develop #1" → Copy riff-1/app.jsx to ./app.jsx, confirm ready for vibes iteration
- "Pick different" → Ask which riff number, then copy that one
- "Turn into SaaS" → Copy riff-1/app.jsx, auto-invoke /vibes:sell skill
- "Deploy demo" → Ask which riff, auto-invoke /vibes:exe skill with that riff's index.html
- "I'm done" → Confirm riffs saved, provide directory listing
