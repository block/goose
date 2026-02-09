---
title: "Level Up Your AI Game with rp-why"
description: "A goose skill that measures the cognitive complexity of your AI collaboration using the Gas Town × DOK framework."
date: 2026-02-06
authors:
  - dakota
---

![rp-why skill banner](rp-why-banner.png)

## What is rp-why?

rp-why is your personal AI collaboration coach. It answers two critical questions:

1. **Are you using the most effective AI tools for your work?**
2. **Are you asking questions that demonstrate cognitive depth?**

Think of it as a fitness tracker for your AI practice—it shows you where you are, where you could be, and how to get there.

**Want the theory?** Check out [Measuring the Cognitive Complexity of Human-AI Collaboration](https://engineering.block.xyz/blog/-gas-town-x-webbs-dok).

**Want to use it?** Keep reading.

<!-- truncate -->

## The Problem

**Without rp-why**

* Teams use powerful AI agents for simple tasks, wasting resources.
* Lack of usage visibility creates blind spots.
* No feedback loop keeps people stuck and stagnant.
* Sophisticated tools get burned on trivial work, killing ROI.

**With rp-why**

* Teams match tools to task complexity, driving real efficiency.
* Usage data makes patterns visible and actionable.
* Smart nudges help people level up continuously.
* Progress is tracked over time, turning effort into measurable growth.

## The Two Dimensions

### Dimension 1: Gas Town Stage (Tool Sophistication)

Where are you on the AI adoption ladder?

| Stage | Level | Description |
|-------|-------|-------------|
| 1-2 | Chatbot Curious | Basic web chatbots, occasional use |
| 3-4 | IDE Integrated | Copilot, chat in your editor |
| 5 | Agent Autonomous | CLI tools like goose running independently |
| 6-8 | Multi-Agent Master | Orchestrating multiple AI agents, agentic workflows |

### Dimension 2: DOK Level (Question Depth)

How complex are your prompts?

| Level | Name | Example Prompts |
|-------|------|-----------------|
| DOK 1 | Recall | "What is X?" "List Y" "Define Z" |
| DOK 2 | Apply | "How would I...?" "Compare A and B" |
| DOK 3 | Strategic | "Design a system for..." "Analyze trade-offs..." |
| DOK 4 | Extended | "Research over multiple sessions..." "Create a framework..." |

---

## The Quadrant Map

Find yourself on the map:

```
                      LOW DOK                    HIGH DOK
                 (Simple Questions)         (Complex Questions)
              ┌────────────────────────┬────────────────────────┐
   HIGH       │                        │                        │
   STAGE      │    UNDERUTILIZING      │       FRONTIER         │
  (Powerful   │                        │                        │
   Tools)     │  You have a Ferrari    │  You're pushing        │
              │  and you're driving    │  boundaries!           │
              │  to the mailbox.       │  Document what you     │
              │  → Level up your       │  learn.                │
              │    questions!          │  → Share your          │
              │                        │    discoveries!        │
              ├────────────────────────┼────────────────────────┤
   LOW        │                        │                        │
   STAGE      │    LEARNING ZONE       │    THINKING AHEAD      │
  (Basic      │                        │                        │
   Tools)     │  Natural starting      │  Your brain exceeds    │
              │  point. Focus on       │  your tools!           │
              │  learning the tools.   │  → Time to upgrade     │
              │  → Try one new         │    your AI toolkit!    │
              │    capability today!   │                        │
              └────────────────────────┴────────────────────────┘
```

**Goal:** Move toward the **FRONTIER** quadrant (top-right)

---

## What the Output Looks Like

```
╔══════════════════════════════════════════════════════════════════╗
║                    SESSION ANALYSIS                              ║
╚══════════════════════════════════════════════════════════════════╝

GAS TOWN STAGE: 5 (Agent Autonomous)

DOK DISTRIBUTION
────────────────────────────────────────────────────────────────────
DOK 1 (Recall):      ████░░░░░░░░░░░░░░░░  17%
DOK 2 (Apply):       ████████████░░░░░░░░  52%
DOK 3 (Strategic):   ██████░░░░░░░░░░░░░░  26%
DOK 4 (Extended):    █░░░░░░░░░░░░░░░░░░░   5%

QUADRANT: Underutilizing
────────────────────────────────────────────────────────────────────
You have a Ferrari and you're driving to the mailbox.
→ Level up your questions!

GROWTH NUDGES
────────────────────────────────────────────────────────────────────
1. Shift 2-3 DOK 2 prompts to DOK 3 by adding "analyze trade-offs"
2. Before simple queries, ask: "Can I make this more strategic?"
3. Try one DOK 4 extended investigation this week

🪞 REFLECTION
────────────────────────────────────────────────────────────────────
What's the most strategic question you could ask right now?
```

---

## Instant Upgrades for Your Prompts

### Transform DOK 1 → DOK 2

| Before | After |
|--------|-------|
| "What is a microservice?" | "How would I decide between microservices and a monolith for my project?" |

### Transform DOK 2 → DOK 3

| Before | After |
|--------|-------|
| "How do I set up CI/CD?" | "Design a CI/CD strategy that balances speed, reliability, and team workflow for a 5-person team." |

### Transform DOK 3 → DOK 4

| Before | After |
|--------|-------|
| "Design a caching strategy" | "Over the next few sessions, help me research, prototype, and document a caching architecture. Start by analyzing our current bottlenecks." |



## Weekly Workflow Integration

### Monday: Fresh Start
- Run `/rp-why init` (or `/rp-why compare` if you already have a baseline)
- Set intention: "This week I'll aim for 30% DOK 3+ prompts"

### Daily: Quick Check
- End each session with `/rp-why current`
- 30 seconds to see your patterns

### Friday: Reflect
- Run `/rp-why compare`
- Celebrate progress, identify next week's focus


## Gamify Your Growth

### Achievements to Unlock

| Badge | Achievement | Criteria |
|-------|-------------|----------|
| 🥉 | Bronze | Reduce DOK 1 prompts below 25% |
| 🥈 | Silver | Achieve 35%+ DOK 3 prompts in a session |
| 🥇 | Gold | Complete a DOK 4 multi-session project |
| 💎 | Diamond | Reach the Frontier quadrant consistently |

### Personal Challenges

| Challenge | Description |
|-----------|-------------|
| "No DOK 1" Day | Every prompt must be DOK 2+ |
| "Strategic Session" | Aim for 50%+ DOK 3 prompts |
| "Deep Dive Week" | One DOK 4 project across 5 sessions |


## Start Now

### Installation

Install the skill:

```bash
npx skills add https://github.com/block/agent-skills --skill rp-why
```

Make sure you have the built-in [Skills extension](/docs/mcp/skills-mcp/) enabled in goose.

### Step 1: Initialize Your Baseline

```
/rp-why init
```

This analyzes your conversation history and creates your personal baseline. Takes ~30 seconds.

### Step 2: Check Your Current Session

```
/rp-why current
```

See how this session compares to your typical patterns. Are you stretching or coasting?

### Step 3: Track Your Progress

```
/rp-why compare
```

Compare today against your baseline. And ask yourself:

> "What's the most strategic question I could ask right now?"

That's the rp-why mindset.


## FAQ

**Q: How long does baseline generation take?**
A: About 30 seconds. It analyzes your available conversation history.

**Q: Will this slow down my workflow?**
A: No! The commands take seconds. Think of it as a quick glance at your fitness tracker.

**Q: What if I'm in the "Underutilizing" quadrant?**
A: That's the most common position for goose users! It means you have powerful tools—now it's time to ask bigger questions.

**Q: How often should I check?**
A: Daily `/rp-why current`, weekly `/rp-why compare`. Takes under a minute total.

**Q: Can I share my progress with my team?**
A: Yes! The output is designed to be shareable. Screenshot or copy the quadrant visualization.

---

## Attribution

- **Gas Town Framework**: Steve Yegge, ["Welcome to Gas Town"](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04) (January 2026)
- **DOK Levels**: Norman Webb (1997)
- **Full Framework Deep-Dive**: [Measuring the Cognitive Complexity of Human-AI Collaboration](#) (Block Engineering Blog)

<head>
  <meta property="og:title" content="Level Up Your AI Game with rp-why" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2026/02/06/rp-why-skill" />
  <meta property="og:description" content="A goose skill that measures the cognitive complexity of your AI collaboration using the Gas Town × DOK framework." />
  <meta property="og:image" content="https://block.github.io/goose/assets/images/rp-why-banner-d3fdd6f674e8e308169e30efe6379735.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io" />
  <meta name="twitter:title" content="Level Up Your AI Game with rp-why" />
  <meta name="twitter:description" content="A goose skill that measures the cognitive complexity of your AI collaboration using the Gas Town × DOK framework." />
  <meta name="twitter:image" content="https://block.github.io/goose/assets/images/rp-why-banner-d3fdd6f674e8e308169e30efe6379735.png" />
</head>