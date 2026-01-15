#!/usr/bin/env node
import React from 'react';
import { render } from 'ink';
import { App } from './App.js';
import { OrchestratorApp } from './OrchestratorApp.js';
import { AcpClient } from './client.js';

const DEFAULT_SERVER_URL = 'http://127.0.0.1:3284';

const args = process.argv.slice(2);
let serverUrl = DEFAULT_SERVER_URL;
let oneShotPrompt: string | null = null;
let orchestratorMode = false;
let repoPath = process.cwd();

for (let i = 0; i < args.length; i++) {
  if ((args[i] === '--server' || args[i] === '-s') && args[i + 1]) {
    serverUrl = args[++i];
  } else if ((args[i] === '--prompt' || args[i] === '-p') && args[i + 1]) {
    oneShotPrompt = args[++i];
  } else if (args[i] === '--orchestrator' || args[i] === '-o') {
    orchestratorMode = true;
  } else if ((args[i] === '--repo' || args[i] === '-r') && args[i + 1]) {
    repoPath = args[++i];
  } else if (args[i] === '--help' || args[i] === '-h') {
    console.log(`
goose-acp-cli - ACP CLI client for goose

Usage: npx tsx src/index.tsx [options]

Modes:
  (default)           Single agent chat mode
  -o, --orchestrator  Multi-agent orchestrator mode

Options:
  -s, --server <url>  Server URL (default: ${DEFAULT_SERVER_URL})
  -p, --prompt <text> One-shot mode: send prompt and exit (single mode only)
  -r, --repo <path>   Repository path for orchestrator (default: current dir)
  -h, --help          Show this help

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
  render(<OrchestratorApp serverUrl={serverUrl} repoPath={repoPath} />);
} else {
  render(<App serverUrl={serverUrl} />);
}

async function runOneShot(serverUrl: string, prompt: string) {
  const client = new AcpClient({ baseUrl: serverUrl });
  
  try {
    await client.connect();
    
    let responseText = '';
    client.onMessage((message) => {
      if (message.method === 'session/update') {
        const params = message.params as any;
        if (params?.update?.sessionUpdate === 'agent_message_chunk') {
          responseText += params?.update?.content?.text || '';
        }
      }
    });
    
    await client.sendRequest('initialize', {
      protocolVersion: '2025-01-01',
      clientInfo: { name: 'goose-acp-cli', version: '1.0.0' },
    });
    
    const { sessionId } = await client.sendRequest<{ sessionId: string }>('session/new', {
      cwd: process.cwd(),
      mcpServers: [],
    });
    
    await client.sendRequest('session/prompt', {
      sessionId,
      prompt: [{ type: 'text', text: prompt }],
    });
    
    console.log(responseText);
    client.disconnect();
    process.exit(0);
  } catch (err) {
    console.error('Error:', err);
    process.exit(1);
  }
}
