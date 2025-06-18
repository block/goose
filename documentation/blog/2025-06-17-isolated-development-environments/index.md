---
title: "Isolated Dev Environments in Goose with container-use"
description: Never worry about breaking your development setup again with containerized, git-branch-isolated development environments powered by container-use
authors:
    - mic
tags: [extensions, containers, development, isolation, mcp, dagger]
---

![Dagger Logo](https://avatars.githubusercontent.com/u/78824383?v=4&s=100)

Over 10 years ago Docker came on the scene and introduced masses of developers to the concept and practice of containers.
These containers were useful to solve deployment and buildtime problems (and in some cases development environments) and rapidly became mainstream. 
Underlying these containers was interesting tech around "copy on write" filesystems and isolated "not quite" virtual machine like environments (certainly helped to contain processes and clean up after).

Dagger (the project and company!) followed on from Docker (by Solomon Hykes, creator of Docker) which furthered the reach of containers for developers.
A project that has come out of that is an MCP server called `container-use` - which provides a cli and tool which can be invaluable for coding agents to able to do work in isolated environments and branches with a clear lifecycle and ability to roll back, limit changes and risk (compared to running an agent direct on your system) but with the same ergonomics and convenience you are used to with agents. 

The container use MCP is still an emerging and changing project and utility, so consider it early days for it but it is moving fast and can and will provide some really useful tools for easy isolation when you need it.


Please take a look at **isolated development environments** in Goose, powered by **[Dagger's Container Use MCP server](https://github.com/dagger/container-use)**. This brings containerized, git-branch-isolated development directly into your Goose workflow.

<!-- truncate -->

## The Problem

Traditional development often means working directly on your local machine, where:

- Dependencies can conflict between projects
- System changes might break other tools
- Experimental code risks your stable codebase
- Cleanup after failed experiments is tedious
- Processes are left running, resources consumed that aren't freed
- Changes are made which can't easily be undone

## The Solution: Isolated Environments

**[Dagger's Container Use](https://github.com/dagger/container-use)** extension gives Goose the ability to work in completely isolated environments:

- **Git branch isolation**: Each experiment gets its own branch
- **Container isolation**: Code runs in clean, reproducible containers
- **Easy reset**: Start fresh anytime without cleanup

## Getting Started

### 1. Install Container Use

**macOS (recommended):**
```bash
brew install dagger/tap/container-use
```

**All platforms:**
```bash
curl -fsSL https://raw.githubusercontent.com/dagger/container-use/main/install.sh | bash
```

### 2. Add to Goose

Click this link to automatically add the extension:

**[🚀 Add Container Use to Goose](goose://extension?cmd=cu&arg=stdio&id=container-use&name=container%20use&description=use%20containers%20with%20dagger%20and%20git%20for%20isolated%20environments)**

Or manually add to `~/.config/goose/config.yaml`:

```yaml
extensions:
  container-use:
    name: container-use
    type: stdio
    enabled: true
    cmd: cu
    args:
    - stdio
    envs: {}
```

## Real-World Use Cases

### Experimenting with New Dependencies

"I want to try adding Redis to this project, but I'm not sure if it's the right fit. Can you set up an isolated environment?"

Goose creates a new git branch, spins up a container with Redis, and lets you experiment. If it doesn't work out, simply exit—no cleanup needed.

### Risky Refactors

"I want to completely restructure this codebase, but need to be able to roll back easily."

Work in an isolated branch and container. If the refactor succeeds, merge it back. If not, delete the branch and container.

### Learning New Technologies

"I want to try this new framework without installing dependencies on my main system."

Experiment in a container with all tools pre-installed, without touching your host system.


## Guide

**[Get started with the full guide →](/docs/guides/isolated-development-environments)**

---

*Questions? Join our [GitHub discussions](https://github.com/block/goose) or [Discord](https://discord.gg/block-opensource). Learn more about Dagger at [dagger.io](https://dagger.io/).*

<head>
  <meta property="og:title" content="Supercharge Your Development with Isolated Environments in Goose" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2025/06/17/isolated-development-environments" />
  <meta property="og:description" content="Never worry about breaking your development setup again with containerized, git-branch-isolated development environments powered by container-use" />
  <meta name="twitter:card" content="summary" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="Supercharge Your Development with Isolated Environments in Goose" />
  <meta name="twitter:description" content="Never worry about breaking your development setup again with containerized, git-branch-isolated development environments powered by container-use" />
</head>