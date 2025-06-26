---
unlisted: true
title: "What's in my .goosehints file (and why it probably shouldn't be)"
description: A deep dive into .goosehints vs Memory Extension, and how to optimize your Goose configuration for better performance
authors:
    - ian
---

![blog cover](blog-banner.png)

# What's in my .goosehints file (and why it probably shouldn't be)

As Goose users, we have two main ways to provide persistent context to our AI assistant: the `.goosehints` file and the [Memory Extension](/docs/mcp/memory-mcp) MCP server. Today, I'll share what's in my `.goosehints` file, why some of it should probably move to the Memory Extension, and how you can make that choice.

<!-- truncate -->

## AI Agents and Memory

Imagine ordering coffee at two different cafes. At the first cafe, you're a first-time customer, carefully explaining "medium mocha latte, fat-free milk, extra hot, no foam, with one pump of vanilla." At your regular coffee spot, though, the barista sees you coming and just says "the usual?"

That stored knowledge – your preferences, quirks, and routine – makes the whole interaction faster and more pleasant for everyone.

This is exactly the challenge we face with AI assistants. By default, they start each conversation (aka, "context window") fresh – no memory of your coding standards, documentation preferences, or how you like your pull requests structured. The same way you'd get tired of reciting your detailed coffee order every morning, it's inefficient to repeatedly explain to your AI assistant that you prefer Python's Black formatter, want detailed commit messages, and or how you want to construct a briefing going to everyone in the company.

This is where persistent context comes in. Through tools like `.goosehints` and the [Memory Extension](/docs/mcp/memory-mcp) MCP server, we can give our AI assistants the equivalent of a barista's "regular customer" knowledge. But just as you wouldn't want your barista memorizing your entire life story just to make your coffee, we need to be thoughtful about what context we make persistent. The key is finding the right balance between having enough context to work efficiently and not overwhelming our systems with unnecessary information.

Let's explore how to strike that balance.

### What is .goosehints?

`.goosehints` is a configuration file that lives in your Goose directory (usually `~/.config/goose/`). It can contain any information that you want Goose to process every time you interact with Goose, providing a foundation for how it interacts with you.

You can read more about `.goosehints` in the [Goose documentation](/docs/guides/using-goosehints).

### What is the Memory Extension?

The [Memory Extension](/docs/mcp/memory-mcp) is a dynamic storage system using the Model Context Protocol that allows you to store and retrieve context on-demand using tags or keywords. It lives in your `~/.goose/memory` directory (local) or `~/.config/goose/memory` (global).

Unlike `.goosehints`, which is static and loaded entirely with every request, Memory Extension can be updated and accessed as needed, allowing for more flexible and user-specific configurations.

## How are .goosehints and Memory Extension used in Goose?

At a very high level, when you have a conversation with Goose, it processes your request in two main steps:

Goose interprets your request to detect tags or keywords needed for possible Memory Extension lookups. Then it loads your entire `.goosehints` file, and sends that, along with any relevant Memory Extension entries to the LLM to generate a response.

![how a user interaction works with Goose](goosehints-vs-memory.png)

## The Implications of .goosehints

To reiterate, **every single line in your `.goosehints` file gets sent with every request to Goose**. If you write a ton of rules and ideas with all your project preferences, coding standards, and workflow documentation and so on into the .goosehints file, the ENTIRE file is being transmitted and processed every time you ask Goose anything, even something as simple as "what time is it?"

This is particularly important for users who are paying for their own LLM access (like ChatGPT or Gemini). Here's why:

- **Input Tokens = Real Money**: Every line in your `.goosehints` file consumes input tokens. The LLM must process these tokens as part of its system instructions before it even looks at your question. While a small `.goosehints` file might not seem like a big deal, it can quickly add up if you're not careful. All-day vibe coding sessions are going to be using lots of tokens just interpreting that whole `.goosehints` file every time.

- **Context Window Impact**: Large `.goosehints` files eat into your [context window](https://zapier.com/blog/context-window/#:~:text=The%20cons%20of%20a%20large%20context%20window%20in%20AI&text=The%20requirements%20to%20process%20AI,request%2C%20things%20quickly%20add%20up.), which is the amount of information the LLM can consider at one time. If your `.goosehints` file is too big, it can push out important context from your actual question or task, leading to less accurate or helpful responses.

This means a large `.goosehints` file can:
- Increase your AI costs
- Reduce the amount of context available for actual work
- Limit the complexity of tasks you can perform

## Where I went wrong with my .goosehints

When I first started using Goose, I treated `.goosehints` like a catch-all for everything I wanted Goose to remember. I included:
- rules on writing outlines for blog posts
- how I like Python code written and formatted
- notes about frontend development
- etc

That meant that every request I would make asking Goose to help come up with a catchy blog title was sending all of my rules about Python and using Tailwind CSS.

### So what "belongs" in .goosehints?

Here's something I end nearly every AI prompt with:

> If you're not 95% sure how to complete these instructions, or that you'll be at least 95% factually accurate, **do not guess or make things up**. Stop and ask me for more information or direction. If you're finding resources online, give me 1 or 2 URLs that informed your response.

I also like to end many of my prompts asking if Goose has any clarifying questions before doing the work I'm attempting:

> Based on the information I've provided, ask me any clarifying questions **before** doing any work, or tell me that you're ready to proceed.

Since these are things that I definitely want to add to every request I make to Goose, I've simplified my .goosehints file to include only these types of rules and standards.

## Everything else got moved into the Memory Extension

The Memory Extension uses a tagging system to remember context based on keywords. You can give Goose a command to "remember" something, and Goose will write a Memory entry with appropriate tags. The next time you ask Goose to do something with Python, it will parse your request, look for relevant tags, and use appropriate Memory entries to send as part of the context for just that request.

So all of my Python rules can be written as a command to Goose like this:

```text
Remember that when I ask about Python, I want to conform to the following standards and guidelines:
- use Python 3.12+ syntax
- use type hints for all function signatures
- use f-strings for string formatting
- use the latest Python features and libraries
- use Flake8 for linting
- use black for formatting
- if I ask to build a CLI based tool, expect to take command line arguments and make a colorful interface using ANSI colors and the rich library
- if I ask to build an API, expect to build a RESTful API use FastAPI and to send back data in JSON format
```

Now, Goose will only send these Python-related rules when I ask it to do something with Python. This is far more efficient.

Here's the resulting Memory file that Goose made:

```text
# python standards development formatting linting api cli
Python Development Standards:
- Python version: 3.12+
- Mandatory type hints for all function signatures
- Use f-strings for string formatting
- Use latest Python features and libraries
- Code formatting: black
- Linting: Flake8
- CLI tools: Use command line arguments and rich library for colorful interface
- APIs: Use FastAPI for RESTful APIs with JSON responses
```

The first line starts with a hash `#` and a space-separated list of keywords and tags that it will use to discern when or whether to retrieve this content to send with a request to my LLM.

## To hint, or not to hint?

While `.goosehints` is powerful, it's important to use it judiciously. By moving appropriate content to the Memory Extension, you can optimize your Goose experience and keep your AI context use more efficient. Remember: every line in `.goosehints` is sent with every request, so choose wisely what goes there.

Share your own `.goosehints` optimization stories in the [Goose community on Discord](http://discord.gg/block-opensource)!

<head>
  <meta property="og:title" content="What's in my .goosehints file (and why it probably shouldn't be)" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2025/06/05/whats-in-my-goosehints-file" />
  <meta property="og:description" content="Learn how to optimize your Goose configuration by understanding when to use .goosehints vs Memory Extension for better performance and maintainability." />
  <meta property="og:image" content="https://block.github.io/goose/assets/images/blog-banner-7f0e5ed1cf875e64e3ebb3250932baaf.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="What's in my .goosehints file (and why it probably shouldn't be)" />
  <meta name="twitter:description" content="Learn how to optimize your Goose configuration by understanding when to use .goosehints vs Memory Extension for better performance and maintainability." />
  <meta name="twitter:image" content="https://block.github.io/goose/assets/images/blog-banner-7f0e5ed1cf875e64e3ebb3250932baaf.png" />
  <meta name="keywords" content="Goose; .goosehints; Memory Extension MCP; AI configuration; performance optimization; developer productivity; context management; AI assistant; token costs; LLM efficiency" />
</head>