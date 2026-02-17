#!/usr/bin/env node
import React from 'react';
import { render } from 'ink';
import { App } from './App.js';
import { OrchestratorApp } from './OrchestratorApp.js';
import { SdkAcpClient } from './acp-client.js';
import type { SessionNotification, TextContent } from '@agentclientprotocol/sdk';

const DEFAULT_SERVER_URL = 'http://127.0.0.1:3284';

const args = process.argv.slice(2);
let serverUrl = DEFAULT_SERVER_URL;
let oneShotPrompt: string | null = null;
let orchestratorMode = false;
let repoPath = process.cwd();
let transportType: 'http' | 'websocket' = 'http';

for (let i = 0; i < args.length; i++) {
  if ((args[i] === '--server' || args[i] === '-s') && args[i + 1]) {
    serverUrl = args[++i];
  } else if ((args[i] === '--transport' || args[i] === '-t') && args[i + 1]) {
    const value = args[++i];
    if (value === 'websocket' || value === 'ws') {
      transportType = 'websocket';
    } else if (value === 'http' || value === 'sse') {
      transportType = 'http';
    } else {
      console.error(`Invalid transport: ${value}. Use 'http' or 'websocket'`);
      process.exit(1);
    }
  } else if ((args[i] === '--prompt' || args[i] === '-p') && args[i + 1]) {
    oneShotPrompt = args[++i];
  } else if (args[i] === '--orchestrator' || args[i] === '-o') {
    orchestratorMode = true;
  } else if ((args[i] === '--repo' || args[i] === '-r') && args[i + 1]) {
    repoPath = args[++i];
  } else if (args[i] === '--help' || args[i] === '-h') {
    console.log(`
goose-acp-tui - ACP TUI client for goose

Usage: npx tsx src/index.tsx [options]

Modes:
  (default)           Single agent chat mode
  -o, --orchestrator  Multi-agent orchestrator mode

Options:
  -s, --server <url>     Server URL (default: ${DEFAULT_SERVER_URL})
  -t, --transport <type> Transport type: http or websocket (default: http)
  -p, --prompt <text>    One-shot mode: send prompt and exit (single mode only)
  -r, --repo <path>      Repository path for orchestrator (default: current dir)
  -h, --help             Show this help

Examples:
  npm start                                    # HTTP/SSE transport (default)
  npm start -- --transport websocket           # WebSocket transport
  npm start -- -t ws                           # WebSocket (short form)
  npm start -- -s http://localhost:8080        # Custom server
  npm start -- -p "Fix the bug"                # One-shot mode
  npm start -- --orchestrator                  # Orchestrator mode

Orchestrator Mode:
  Manage multiple goose agents working on different tasks.
  Each task runs in its own git worktree for isolation.

  Controls:
    n         Create new workstream
    ↑/↓       Navigate workstreams
    Enter     Focus on workstream
    s         Stop workstream
    q         Quit
    ?         Help

Single Agent Mode:
  Interactive chat with a single goose agent.

  Controls:
    Type and press Enter to send messages
    Ctrl+C    Quit
`);
    process.exit(0);
  }
}

if (oneShotPrompt) {
  runOneShot(serverUrl, oneShotPrompt);
} else if (orchestratorMode) {
  render(<OrchestratorApp serverUrl={serverUrl} repoPath={repoPath} transportType={transportType} />);
} else {
  render(<App serverUrl={serverUrl} transportType={transportType} />);
}

async function runOneShot(serverUrl: string, prompt: string) {
  let responseText = '';
  
  // Create SDK client with session update handler
  const client = new SdkAcpClient(
    { serverUrl },
    {
      onSessionUpdate: (notification: SessionNotification) => {
        const update = notification.update;
        if (update.sessionUpdate === 'agent_message_chunk' && update.content?.type === 'text') {
          responseText += (update.content as TextContent).text || '';
        }
      }
    }
  );
  
  try {
    // Connect and initialize session
    await client.connect();
    
    // Send the prompt and wait for response
    await client.prompt(prompt);
    
    // Print the response
    console.log(responseText);
    
    client.disconnect();
    process.exit(0);
  } catch (err) {
    console.error('Error:', err);
    process.exit(1);
  }
}
