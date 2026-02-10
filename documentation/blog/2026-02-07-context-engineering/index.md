---
title: "One Shot Prompting is Dead - The Era of Context Engineering"
description: "One-shot prompting isn’t the future. Context engineering is. A practical look at how AI workflows are shifting from clever prompts to engineered systems."
authors:
  - ebony
---

![One-shot prompting is dead - The era of context engineering](blogbanner.png)

I attended one shot prompting’s funeral.

There were no tears. Just a room full of developers quietly pretending they weren’t taking shots the night before. Because if we’re being honest, everyone saw this coming and couldn’t be happier it was over.

Saying “one-shot prompting is dead” isn’t revolutionary. It’s just catching up to what builders have been experiencing for months.

<!-- truncate -->

---

## The blog post that aged faster than oat milk

Earlier this year I wrote a post about  [how to prompt better](https://block.github.io/goose/blog/2025/03/19/better-ai-prompting). I shared tricks, phrasing tips, and even said to add a few “pleases” and “thank yous” and your AI agent would give you the world. At the time it felt cutting edge, because it was. There were livestreams and conference talks entirely about how to prompt better.

Less than a year later, it feel… quaint. Not because prompting stopped mattering, but because prompting stopped being the main character.

The conversation shifted from:

> “How do I ask the model better questions?”

to

> “What environment am I dropping this model into?”

That’s a completely different problem, and now it has a name. **Context engineering**.


---

## The abstraction that broke

One-shot prompting worked when models were chat toys. You asked a clever question, you got a clever answer, and by “clever answer” I mean a fully “working” app, so everyone clapped. But the moment we asked agents to plan, remember, call tools, and operate across multiple steps, the definition of “worked” fell apart.

A single prompt stopped being a solution and became a bottleneck. What matters now isn’t the sentence you type. It’s the system that surrounds it.

Memory and long-term state. Tools and retrieval. Structured constraints. Reusable skills. Planning loops. Guardrails. Context handoffs between steps.

Once we started wanting deployable and repeatable workflows, prompts became just one input among many.

As someone put it in a thread I recently came across:

> “The best model with bad context loses to an average model with great context.”

That’s the shift in one line.


---

## We’re already living in the post-prompt era

This isn’t a prediction. It’s already happening.

Patterns like [Ralph Wiggum loop](https://ghuntley.com/loop/), [OpenClaw](https://openclaw.ai/), and multi-agent planning systems aren’t about clever wording. They’re about designing context pipelines that let agents think across steps.

The reaction to projects weren't just hype. Developers went a **little* feral over them because they were hungry for real examples of context engineering. Not prompt tricks but actual systems that held state, iterated, and behaved *predictably* across time.

That excitement tells you where the energy is moving. Builders are asking for environments that scale. And once you start designing those environments, new concerns show up immediately, context pollution, memory drift, permission boundaries, security, governance, etc. Context engineering isn’t just about giving agents more power, it’s about deciding what they’re allowed to carry forward and what they’re not allowed to touch.

That’s architecture and it’s why this shift matters.

You see the same philosophy showing up across the ecosystem. Stripe’s [end-to-end coding agents](https://stripe.dev/blog/minions-stripes-one-shot-end-to-end-coding-agents), Goose’s [skills](https://block.github.io/goose/docs/guides/context-engineering/using-skills/) and [recipes](https://block.github.io/goose/docs/guides/recipes/), and tools like [rp-why](https://block.github.io/goose/blog/2026/02/06/rp-why-skill) that analyze interaction patterns over time are all converging on the same idea. Success isn’t measured in prompts anymore, it’s measured in systems and how well you can orchestrate them.

Developers are already sharing success stories with context engineering. My coworker, Rizel Scarlett, documented how she used [RPI to build a lightweight OpenClaw alternative](https://block.github.io/goose/blog/2026/02/06/rpi-openclaw-alternative) after simple back and forth prompting led her to hit multiple context limits. Structured [research, planning, and implementation](https://block.github.io/goose/docs/tutorials/rpi/) gave her agent room to reason instead of react.

Once you see that, the next question isn’t philosophical anymore. It’s practical. What does building this way actually look like for me?

---

## What building inside this shift actually looks like

All of this sounds philosophical until you try to ship something real.

When I started building a skills marketplace, one-shot prompting became messy. Distributing context across disconnected workflows didn’t hold. The work forced me to experiment with RPI.

I didn’t adopt a new context engineering workflow because it was trendy. I adopted it because the alternative kept breaking. Once I committed to RPI, the friction disappeared. I repeated myself less. My agent made fewer mistakes. We stopped losing the plot halfway through the project. And I had research and planning docs I could actually read.

That shift didn’t apply to just one project. I’ve started applying the same mindset to other workflows too, encoding operational processes into skills and recipes with the help of subagents so operational memory lives in the system and not in people’s heads. When context is handled by design, people get to focus on creative decisions instead of procedural ones.

I wrote a short post about the marketplace shift [on LinkedIn](https://www.linkedin.com/feed/update/urn:li:activity:7423057839969062912/). I’m breaking down the full build soon, because applying context engineering in practice changes how you leverage these agent workflows entirely.


---

## This is good news for people who think beyond code

The interesting part is this shift isn’t just technical. There’s a quiet career implication hiding inside it. AI isn’t replacing engineers. It’s replacing the old workflows and fundamentals we’ve relied on.

The people being phased out aren’t all developers. In my opinion, it’s the ones whose thinking ends at “my code runs, I’m done.” Context engineering rewards a different mindset. It’s more about judgment calls, knowing what to build, what to constrain, and what the system should optimize for.

We need to understand how decisions propagate through a system and the consequences they create. That’s a mindset shift I’m actively working toward too, so I’d be remiss not to share it. It’s about zooming out, seeing the bigger picture, and designing with downstream effects in mind.

---

## The new skill isn’t prompting

Prompting isn’t gone. It’s still useful for demos and bringing ideas to life quickly. 

But one-shot prompting as a workflow has been demoted, because the long term skill to master is context design. The real work now is understanding how information flows, what persists, what gets retrieved, what’s reusable, and what the agent is allowed to safely assume.

One-shot prompting didn’t die because it failed. It died because we outgrew it.

<head>
  <meta property="og:title" content="One Shot Prompting is Dead - The Era of Context Engineering" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog//2026/02/07/context-engineering" />
  <meta property="og:description" content="One-shot prompting isn’t the future. Context engineering is. A practical look at how AI workflows are shifting from clever prompts to engineered systems." />
  <meta property="og:image" content="https://block.github.io/goose/assets/images/blogbanner-2fa90c93a49496447d38217739242dec.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="One Shot Prompting is Dead - The Era of Context Engineering" />
  <meta name="twitter:description" content="One-shot prompting isn’t the future. Context engineering is. A practical look at how AI workflows are shifting from clever prompts to engineered systems." />
  <meta name="twitter:image" content="https://block.github.io/goose/assets/images/blogbanner-2fa90c93a49496447d38217739242dec.png" />
</head>