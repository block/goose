---
title: "Gastown Explained: How to Use Goosetown for Parallel Agentic Engineering"
description: "Learn how Gastown and Goosetown lead the industrial coding revolution by teaching AI agents to work together in a team. This beginner guide explains the infrastructure we're using to move from talking to one AI to coordinating many agents at once."
authors:
  - rizel
  - tyler
---

![Goosetown](goosetown.png)

On New Year's Day 2026, while many were recovering from the night before, a different kind of hangover took hold of every AI-pilled, chronically online software engineer. Steve Yegge published a new blog post: "[Welcome to Gas Town](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04)." Some walked away inspired to finally use their agents optimally; others were just plain confused. If you're like me, you felt a bit of both. 

Yegge's 34 minute post is a sprawling vision filled with futuristic ideas, playful characters, and enough side tangents to make your head spin. But underneath the lore is a massive architectural shift. I want to take a step back and simplify the "Big Idea" for everyone: Gas Town is a philosophy and a proof of concept to help people coordinate multiple agents working together.

<!-- truncate -->

## The Paradigm Shift of Agentic Engineering

Most people use AI agents sequentially. The workflow can look like this:

* **1:00 PM:** You: "goose, build the API endpoint."  
* *[Wait 10 minutes, check back.]*  
* **1:10 PM:** You: "Now build the frontend."  
* *[Wait 10 minutes, check back.]*  
* **1:20 PM:** You: "Now write the tests."  
* *[Wait 10 minutes, check back.]*  
* **1:30 PM:** Project complete.

You've built a project in 30 minutes, which is fast, but you spent most of that time just watching a progress bar. Some engineers started to realize that if we are running one agent, we can run another five at the same time. 

For example, Agent A builds the API, Agent B can start the frontend, Agent C can write tests, and Agent D can investigate a bug in that legacy codebase you've been avoiding.

This is how people are buying their time back. They're getting entire sprints done in an hour by running parallel threads. (Just don't tell your boss because the reward for finishing work is always more work.)

However, since agents don't communicate with each other, this approach introduces new problems:

* **Merge conflicts**: Two agents change the same line in the same file and break everything.  
* **Lost context**: Sessions crash or the agent starts hallucinating because it's been talking too long, and suddenly an hour of "work" vanishes.  
* **Human bottleneck**: You end up constantly checking your phone at a party on the weekend or in bed to see if your agents are still on track making you a babysitter for agents.

## Gas Town Explained

Gas Town is designed to stop the babysitting. It coordinates the distribution of tasks among parallel agents so you don't have to. The system uses:

* **Worktrees**: These automatically put each agent in its own separate workspace so they don't step on each other's toes.  
* **Beads**: It uses beads to track progress. If a session crashes, the next agent session can pick up exactly where the last agent left off.  
* **Communication**: Each agent reports aloud what it's up to or observing, so other agents gain the necessary context. 

This system also introduces a cast of characters:

* **The Mayor**: your main agent interface that coordinates all the other agents  
* **The Polecat(s)**: These are worker agents. They work on separate work trees, and take instruction from the Mayor.  
* **The Witness**: Observes the worker agents, nudges them when they get stuck, and escalates issues to keep the system running

I won't list every single character here (it gets deep), but the takeaway is: Gas Town creates a chain of command with a shared way to communicate.

## Introducing Goosetown

This is exactly the kind of futuristic thinking we're building toward at [goose](https://block.github.io/goose). So the goose team, specifically Tyler Longwell, built our own take on this called [Goosetown](https://github.com/block/goosetown).

Goosetown is a multi-agent orchestration layer built on top of goose. Like Gas Town, it coordinates parallel agents. Unlike Gas Town, it's deliberately minimal and built for research-first parallel work.

When you give Goosetown a task, the main agent acts as an Orchestrator, breaking the job into phases: research, build, and review. Then, it spawns parallel delegates to get it done. Each delegate communicates via a shared Town Wall, an append-only log where every agent posts what they're doing and what they've found.

Here's a real Town Wall snippet from a session where parallel researchers converged on a pivot quickly:

* **[10:14] researcher-api-auth** - 🚨 POTENTIAL SHOWSTOPPER: Service callers have EMPTY capabilities. Planned auth path will silently reject every request. This needs a code change, not just config.   
* **[10:14] researcher-endpoints** - 💡 Found: native endpoint already exists with minimal deps. Alternative path viable.   
* **[10:15] researcher-source** - ✅ Done. Confirmed: native path requires zero new dependencies. Recommending pivot.

Goosetown operates on 4 components: [skills](/docs/guides/context-engineering/using-skills), [subagents](/docs/mcp/summon-mcp#subagents), beads, and a gtwall.

### Skills

[Skills](/docs/guides/context-engineering/using-skills) are Markdown files that describe how to do something like "how to deploy to production." Goosetown uses these to tell each Delegate how to do its specific job. When a Delegate spawns, it's "pre-loaded" with the skill for its role (Orchestrator, Researcher, Writer, Reviewer).

### Subagents

Instead of doing everything in one long conversation that eventually hits a "context cliff," Goosetown uses [subagents](/docs/mcp/summon-mcp#subagents), ephemeral agent instances. These are triggered by the [summon extension](/docs/mcp/summon-mcp), using `delegate()` to hand off work to a fresh agent instance. They do the work in their own clean context and return a summary, keeping your main session fast and focused.

### Beads

Goosetown uses Beads to track progress so work survives crashes. It's a local issue tracker based on Git. The Orchestrator creates issues, delegates update them, and if a session fails, the next agent picks up the "bead" and continues the work.

### gtwall

gtwall is an append-only log that delegates use to communicate and coordinate. All delegates post and read activity.

## A Note from the Creator 

> *I mostly use Goosetown when I'm trying to answer something that has a lot of independent angles, where missing a constraint is more expensive than spending extra tokens. For example, integration research ('how does system X actually authenticate?') or migration planning ('what would break first if we moved?')*
>
> *There's a real tax to running multiple agents. If I can describe the task in one paragraph and I already know where to start, I don't need Goosetown. Parallelism improves throughput, but it adds coordination overhead.*
>
> *The next improvements I care about are mostly about making failure modes cheaper. The system already has turn limits and a flat hierarchy, but it doesn't yet have good cost controls beyond that. Token budgets and basic circuit breakers would make it harder for a delegate to burn a surprising amount of compute in a tight loop.*
>
> *On the coordination side, I'm interested in adding a little more structure without turning it into a framework. Even lightweight conventions, like consistent prefixes on wall messages or a clearer artifact layout, can reduce synthesis work.*
>
> *Longer term, goose has roadmap work around more structured agent-to-agent communication. If that lands, it might replace parts of the wall. The tradeoff is the one we've been making all along: structure buys you scale and tooling; simplicity buys you debuggability and the ability to change policy by editing a markdown file.*
>
> — Tyler Longwell

## Get Started

Ready to try parallel agentic engineering for yourself? [Goosetown](https://github.com/block/goosetown) is open source and available on GitHub. Clone the [repo](https://github.com/block/goosetown), follow the setup instructions in the README, and you'll be orchestrating multiple agents in no time. If you're new to this workflow, watching the video below is a great way to see what a real session looks like before diving in.

<iframe class="aspect-ratio" src="https://www.youtube.com/embed/H2hJjNmvEEA" title="Rizel's first time using Goosetown" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

<head>
  <meta property="og:title" content="Gastown Explained: How to Use Goosetown for Parallel Agentic Engineering" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2026/02/19/gastown-explained-goosetown" />
  <meta property="og:description" content="Learn how Gastown and Goosetown lead the industrial coding revolution by teaching AI agents to work together in a team. This beginner guide explains the infrastructure we're using to move from talking to one AI to coordinating many agents at once." />
  <meta property="og:image" content="https://block.github.io/goose/assets/images/goosetown-6e1bda1a4bd160c0c01cfc58c118492e.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="Gastown Explained: How to Use Goosetown for Parallel Agentic Engineering" />
  <meta name="twitter:description" content="Learn how Gastown and Goosetown lead the industrial coding revolution by teaching AI agents to work together in a team." />
  <meta name="twitter:image" content="https://block.github.io/goose/assets/images/goosetown-6e1bda1a4bd160c0c01cfc58c118492e.png" />
</head>
