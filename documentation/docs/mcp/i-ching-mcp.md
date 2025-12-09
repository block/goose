---
title: I Ching Extension
description: Add I Ching MCP Server as a goose Extension
---

This tutorial covers how to add the [I Ching MCP Server](https://github.com/threemachines/i-ching) as a goose extension to enable traditional I Ching divination readings.

:::tip TLDR
1. Install: `cargo install i-ching`
2. Configure: [Add I Ching Extension](goose://extension?cmd=i-ching-mcp-server&id=i-ching&name=I%20Ching&description=I%20Ching%20divination%20readings)
:::

## Installation

This extension requires Rust 1.85.1 or later. Check your version:

```bash
rustc --version
```

If you need to install or update Rust, visit [rustup.rs](https://rustup.rs/).

:::tip Use goose to Help!
If you're unsure about any of these steps, you can ask goose to help you check your system and guide you through the Rust installation process.
:::

Once Rust is ready, install the I Ching crate from crates.io:

```bash
cargo install i-ching
```

Then click the link above to configure it in goose.

## Example Usage

### goose Prompt

> Our team is facing a challenging decision about our product roadmap. Could you consult the I Ching for some perspective on how to approach this choice?

### goose Output

:::note Desktop
I'd be happy to consult the I Ching for guidance on your team's product roadmap decision. Let me cast a hexagram for you using the traditional three coins method.

─── cast_hexagram | iching ──────────────────────────


Now let me get a detailed interpretation of this reading for your situation:

─── interpret_reading | iching ─────────────────────────

changing_lines:  
&#45;  
    5  
hexagram: 29  
transformed_hexagram: 13


#### I Ching Guidance for Your Product Roadmap Decision

The oracle has given you **Hexagram 29 - Darkness (The Abysmal)** with a changing line in the 5th position, transforming to **Hexagram 13 - Seeking Harmony (Fellowship with Men)**.

**Key Insights for Your Team's Situation:**

**Current Challenge (Hexagram 29):**
- Your team is facing what the I Ching recognizes as a "repetition of danger" - this suggests you're in a challenging situation that requires careful navigation
- Like water in a ravine, you need to find the natural path forward rather than forcing your way through
- The emphasis is on **sincerity and thoroughness** - whatever decision you make, commit to it fully and execute it properly

**Strategic Approach:**
1. **Flow Like Water**: Don't try to overcome obstacles through force. Instead, find the path of least resistance that still moves you toward your goal
2. **Fill Every Depression**: Address all the foundational issues before moving forward. Don't skip steps or leave gaps in your planning
3. **Consistency Over Ambition**: The changing line warns against being "too ambitious." Focus on steady, sustainable progress rather than dramatic leaps

**The Transformation (Hexagram 13):**
The situation naturally evolves toward **Fellowship with Men** - suggesting that the key to resolving your roadmap challenge lies in:
- **Building consensus** among team members
- **Open collaboration** rather than closed-door decisions
- **Crossing the great water** - taking on significant challenges, but doing so together

**Practical Recommendations:**

1. **Acknowledge the Difficulty**: Don't minimize the complexity of your roadmap decision. Accept that it's genuinely challenging.

2. **Seek Team Alignment**: The transformation to "Fellowship" suggests your answer lies in bringing the team together rather than having leadership decide alone.

3. **Take the Conservative Path**: The 5th line specifically advises against overambition. Choose the roadmap option that gets you "out of danger" rather than the most aggressive one.

4. **Focus on Execution**: Once you decide, commit fully. The hexagram emphasizes that success comes from thorough implementation, not perfect strategy.

The I Ching suggests this challenging decision will ultimately strengthen your team's unity and collaboration if approached with patience, sincerity, and collective wisdom.
:::