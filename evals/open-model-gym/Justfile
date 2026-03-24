# Agent Runner - Test Suite (supports goose and opencode)

# Default recipe
default: run

# Full test run - all scenarios, all agents, 3 repetitions (worst kept)
run: _install
    cd suite && npm run test

# Quick test - file-editing + everyday-app-automation, single run each (no repetition)
test: _install
    cd suite && npx tsx src/runner.ts --scenario=file-editing,everyday-app-automation --run-count=1

# Run a specific scenario (all agents, 3 reps)
scenario name: _install
    cd suite && npx tsx src/runner.ts --scenario={{name}}

# Run against a specific agent (all scenarios, 3 reps)
agent name: _install
    cd suite && npx tsx src/runner.ts --agent={{name}}

# Open report in browser
report:
    open report.html

# Install all dependencies
install:
    cd suite && npm install
    cd mcp-harness && npm install && npm run build
    @# Install pi-mcp-adapter for Pi runner MCP support
    @pi list 2>/dev/null | grep -q "pi-mcp-adapter" || pi install npm:pi-mcp-adapter

# Build TypeScript
build: _install
    cd suite && npm run build

# Clear the test cache
clear-cache:
    cd suite && npx tsx src/runner.ts --clear-cache

# Run tests ignoring cache (force fresh runs)
run-fresh: _install
    cd suite && npx tsx src/runner.ts --no-cache

# Show cache stats
cache-stats:
    @if [ -f suite/.cache/index.json ]; then \
        echo "Cache entries: $$(cat suite/.cache/index.json | grep -o '"[a-f0-9]\{16\}":' | wc -l | tr -d ' ')"; \
        echo "Cache size: $$(du -sh suite/.cache 2>/dev/null | cut -f1 || echo '0')"; \
    else \
        echo "No cache found"; \
    fi

# Internal: install if node_modules missing, always rebuild mcp-harness
_install:
    @[ -d suite/node_modules ] || (cd suite && npm install)
    @[ -d mcp-harness/node_modules ] || (cd mcp-harness && npm install)
    @cd mcp-harness && npm run build
    @# Ensure pi-mcp-adapter is installed for Pi runner
    @pi list 2>/dev/null | grep -q "pi-mcp-adapter" || pi install npm:pi-mcp-adapter
