---
draft: false
title: "Running Goose in Containerized Environments"
description: "Learn how to run Goose AI agent in Docker containers for better reproducibility, security, and scalability."
date: 2025-10-03
authors:
  - Agasta
---

![Running Goose in Containers](thumbnail.jpg)

# Running Goose in Containers (Without Losing Your Mind)

I’m a huge fan of containers. They’re not just a cool buzzword for résumés; they actually save your sanity. And today, I’ll show you how to run **Goose**, inside Docker. Once you containerize Goose, you’ll never go back to raw installs.

---

## Why Even Bother?

Look, [Goose](https://block.github.io/goose/) is powerful. It's an AI agent that can automate engineering tasks, build projects from scratch, debug code, and even orchestrate complex workflows. The secret sauce is the [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro) (MCP), which allows Goose to execute actions by connecting to external tools and APIs. Containerizing this powerful, modular system gives you:

- **Reproducibility:** Same setup across dev, staging, and prod

- **Isolation:** No more conflicts with your local Python/Node/Rust installations

- **Scalability:** Easy to deploy multiple instances for CI/CD pipelines

- **Security:** Run as non-root user with minimal attack surface

- **Portability:** Works on any machine with Docker installed

In my experience, containerizing AI tools is especially crucial because LLM providers and API keys need to be handled securely, and containers make that a breeze.

## Quick Start: Pull and Run

The easiest way to get started? Use the pre-built images from GitHub Container Registry. No building required.

```bash
# Pull the latest image
docker pull ghcr.io/block/goose:latest

# Check it's working
docker run --rm ghcr.io/block/goose:latest --version

# Run your first command
docker run --rm \
    -e GOOSE_PROVIDER=openai \
    -e GOOSE_MODEL=gpt-4o \
    -e OPENAI_API_KEY=$OPENAI_API_KEY \
    ghcr.io/block/goose:latest run -t "Hello, containerized world!"
```

Boom! You're up and running. That ~340MB image has everything Goose needs, optimized for size and performance.

## Building Your Own Images

Sometimes you need customizations. Maybe you want the bleeding edge from source, or you need additional tools. Building from source is straightforward:

```bash
# Clone and build
git clone https://github.com/block/goose.git
cd goose
docker build -t goose:local .
```

The build uses multi-stage magic: compiles with Rust's heavy toolchain, then copies just the binary to a minimal Debian runtime. Smart stuff. The Dockerfile even includes Link-Time Optimization (LTO) and binary stripping to keep things lean.

Pro tip: For development builds with debug symbols, add `--build-arg CARGO_PROFILE_RELEASE_STRIP=false`.

## Running Goose Effectively

### Basic CLI Usage

Mount your workspace and let Goose work its magic:

```bash
docker run --rm \
    -v $(pwd):/workspace \
    -w /workspace \
    -e GOOSE_PROVIDER=openai \
    -e GOOSE_MODEL=gpt-4o \
    -e OPENAI_API_KEY=$OPENAI_API_KEY \
    goose:local run -t "Analyze this codebase"
```

### Interactive Sessions

For longer sessions, use the interactive mode:

```bash
docker run -it --rm \
    -v $(pwd):/workspace \
    -w /workspace \
    -e GOOSE_PROVIDER=anthropic \
    -e GOOSE_MODEL=claude-3-5-sonnet-20241022 \
    -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
    goose:local session
```

### Docker Compose for Complex Setups

When you're dealing with multiple services or persistent config, use Docker Compose:

```yaml
version: "3.8"
services:
  goose:
    image: ghcr.io/block/goose:latest
    environment:
      - GOOSE_PROVIDER=${GOOSE_PROVIDER:-openai}
      - GOOSE_MODEL=${GOOSE_MODEL:-gpt-4o}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - ./workspace:/workspace
      - goose-config:/home/goose/.config/goose
    working_dir: /workspace
    stdin_open: true
    tty: true

volumes:
  goose-config:
```

Run with: `docker-compose run --rm goose session`

## Configuration Deep Dive

Goose supports all the usual environment variables: `GOOSE_PROVIDER`, `GOOSE_MODEL`, and provider-specific keys. The container runs as a non-root user (UID 1000) by default, which is great for security.

For persistent config, mount the config directory:

```bash
docker run --rm \
    -v ~/.config/goose:/home/goose/.config/goose \
    goose:local configure
```

Need extra tools? The image is based on Debian Bookworm Slim, so you can install what you need:

```bash
FROM ghcr.io/block/goose:latest

USER root

RUN apt-get update && apt-get install -y vim tmux && rm -rf /var/lib/apt/lists/*

USER goose
```

## CI/CD Integration: Where Containers Shine

This is where containerization really pays off. In CI/CD pipelines, you want consistent, isolated environments. Here's how to integrate Goose:

### GitHub Actions

```yaml
jobs:
  analyze:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/block/goose:latest
      env:
        GOOSE_PROVIDER: openai
        GOOSE_MODEL: gpt-4o
        OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
    steps:
      - uses: actions/checkout@v4
      - name: Run goose analysis
        run: |
          goose run -t "Review this codebase for security issues"
```

### GitLab CI

```yaml
analyze:
  image: ghcr.io/block/goose:latest
  variables:
    GOOSE_PROVIDER: openai
    GOOSE_MODEL: gpt-4o
  script:
    - goose run -t "Generate documentation for this project"
```

I've used this setup on multiple projects, and it's a game-changer for automated code reviews and documentation generation.

## Troubleshooting: Common Gotchas

### Permission Issues

If you see permission errors with mounted volumes, match the user ID:

```bash
docker run --rm \
    -v $(pwd):/workspace \
    -u $(id -u):$(id -g) \
    goose:local run -t "List files"
```

### API Key Headaches

Make sure your environment variables are set correctly. Use `--env-file` for multiple keys:

```bash
echo "OPENAI_API_KEY=your-key-here" > .env
docker run --rm --env-file .env goose:local run -t "Test command"
```

### Network Access

For local services, use host networking:

```bash
docker run --rm --network host goose:local
```

## Advanced Patterns

### Resource Limits

Set memory and CPU limits for production:

```bash
docker run --rm \
    --memory="2g" \
    --cpus="2" \
    goose:local
```

### Custom Entrypoints

Need to debug? Override the entrypoint:

```bash
docker run --rm -it --entrypoint bash goose:local
```

### Multi-Platform Builds

For deployment across architectures:

```bash
docker buildx build --platform linux/amd64,linux/arm64 -t goose:multi .
```

## Production Considerations

For production deployments:

1. **Use specific tags**: Avoid `latest` for reproducibility

2. **Secrets management**: Use Docker secrets or external secret stores for API keys

3. **Logging**: Configure log aggregation

4. **Monitoring**: Set up health checks and metrics

5. **Scaling**: Use orchestration tools like Kubernetes for multiple instances

## Wrap-Up

Containerizing Goose has been one of those "why didn't I do this sooner?" moments in my career. It eliminates environment drift, simplifies deployments, and makes your AI-powered workflows truly portable. So, start with the prebuilt image, then customize as you grow. Your future self will thank you.
