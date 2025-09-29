---
title: "How I Use goose to Plan My Week with Asana and Google Calendar MCPs"
description: Use MCPs with Goose to automate task management and enhance productivity.
authors: 
    - angie
---

![blog cover](mcp-planner.png)

Mondays are overwhelming. A pile of unfinished tasks from last week, new priorities rolling in, and meetings scattered across the calendar. It’s a lot 😩. Instead of manually sorting through my todos and figuring out where everything fits, I use a couple of handy MCP servers with goose and let it figure out my week.

<!--truncate-->

There's so many amazing MCP servers out there to make my work life better, including [Asana](https://github.com/roychri/mcp-server-asana) and [Google Calendar](https://www.pulsemcp.com/servers?q=google+calendar). I added these as goose extensions, which means Goose can now can pull in my tasks, analyze them, and schedule them, all with one simple prompt:

> _**Goose, pull all uncompleted tasks assigned to me in Asana. Group them by type of work to reduce context switching. Estimate how long each task will take. Then, schedule each task accordingly in my Google Calendar.  Make sure not to double book or overload any single day.**_


:::info
I used GPT-4o for this task
:::

With this prompt, Goose reviews my uncompleted tasks in Asana (note that I have my workspace, project, and user IDs stored in [memory](/docs/mcp/memory-mcp)).

Rather than bouncing between different types of work, which is a productivity killer, Goose sorts my tasks into categories based on context. For example:

* Writing-related tasks (blog posts, documentation, emails)
* Async collaboration (PR reviews, providing feedback)
* Technical work (coding, etc)

By grouping similar tasks, I can stay in the right headspace without constantly switching gears.

Goose then estimates how long each task will take, the complexity of the task, and any deadlines. If I need to manually adjust something, I can, but it’s usually pretty spot on.

With my tasks organized and estimated, Goose finds open time slots in my Google Calendar and automatically schedules them. It avoids my meetings and ensures I’m not overloading any single day.

Within the first few minutes of the start of my week, my schedule is already mapped out, optimized for focus.

This has been so extremely helpful in increasing my productivity. Thanks, Goose! 🚀



<head>
  <meta property="og:title" content="MCP in Action: How I Use AI to Plan My Week with Goose, Asana, and Google Calendar" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2025/03/20/asana-calendar-mcp" />
  <meta property="og:description" content="Use MCPs with Goose to automate task management and enhance productivity." />
  <meta property="og:image" content="http://block.github.io/goose/assets/images/mcp-planner-761303c5ddcd5c79ed853536e3f87bcf.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="MCP in Action: How I Use AI to Plan My Week with Goose, Asana, and Google Calendar" />
  <meta name="twitter:description" content="Use MCPs with Goose to automate task management and enhance productivity." />
  <meta name="twitter:image" content="http://block.github.io/goose/assets/images/mcp-planner-761303c5ddcd5c79ed853536e3f87bcf.png" />
</head>