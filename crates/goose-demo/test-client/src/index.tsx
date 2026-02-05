#!/usr/bin/env tsx
/**
 * goose2 test client TUI
 *
 * A clean ACP client for testing ACP-compatible agents.
 * Uses the official @agentclientprotocol/sdk for protocol handling.
 */

import React, { useState, useEffect, useCallback, useRef } from "react";
import { render, Box, Text, useInput, useApp, useStdout } from "ink";
import TextInput from "ink-text-input";
import Spinner from "ink-spinner";
import { spawn } from "node:child_process";
import { Writable, Readable } from "node:stream";

import * as acp from "@agentclientprotocol/sdk";
import { parseArgs, printHelp } from "./config.js";

// ============================================================================
// Types
// ============================================================================

/** A single item in the conversation history */
type HistoryItem =
  | { type: "user"; content: string; timestamp: number }
  | { type: "assistant"; content: string; promptTime: number; firstChunkTime?: number; endTime?: number; chunkCount: number }
  | { type: "tool_call"; id: string; title: string; status: string; result?: string; callTime: number; resultTime?: number };

type AppState =
  | { type: "connecting" }
  | { type: "ready" }
  | { type: "prompting" }
  | { type: "permission"; request: acp.RequestPermissionRequest; respond: (r: acp.RequestPermissionResponse) => void }
  | { type: "error"; message: string };

// ============================================================================
// Helpers
// ============================================================================

/** Format duration in ms to a human-readable string */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

// ============================================================================
// Components
// ============================================================================

interface HistoryViewProps {
  items: HistoryItem[];
  streamingText: string;
  promptSentTime: number | null;
  firstChunkTime: number | null;
  chunkCount: number;
}

/** Wrap text to fit within width, preserving indentation on wrapped lines */
function wrapText(text: string, maxWidth: number, indent: string = ""): string[] {
  if (text.length <= maxWidth) return [text];
  
  const lines: string[] = [];
  let remaining = text;
  let isFirstLine = true;
  
  while (remaining.length > 0) {
    const availableWidth = isFirstLine ? maxWidth : maxWidth - indent.length;
    
    if (remaining.length <= availableWidth) {
      lines.push(isFirstLine ? remaining : indent + remaining);
      break;
    }
    
    // Find a good break point (space) within the available width
    let breakPoint = remaining.lastIndexOf(" ", availableWidth);
    if (breakPoint <= 0) {
      // No space found, force break at width
      breakPoint = availableWidth;
    }
    
    const line = remaining.slice(0, breakPoint);
    lines.push(isFirstLine ? line : indent + line);
    
    // Skip the space we broke at
    remaining = remaining.slice(breakPoint).trimStart();
    isFirstLine = false;
  }
  
  return lines;
}

function HistoryView({ items, streamingText, promptSentTime, firstChunkTime, chunkCount }: HistoryViewProps) {
  const { stdout } = useStdout();
  const terminalWidth = stdout?.columns || 80;
  const contentWidth = terminalWidth - 4; // Account for padding

  // Build all output lines
  // timingSuffix is used for tool calls to render timing in dim at the end
  const allLines: Array<{ text: string; color?: string; dim?: boolean; timingSuffix?: string }> = [];

  for (let i = 0; i < items.length; i++) {
    const item = items[i];

    if (item.type === "user") {
      allLines.push({ text: "" }); // blank line before user message
      // Wrap user input with indentation matching the prompt
      const wrappedUser = wrapText(`❯ ${item.content}`, contentWidth, "  ");
      for (const line of wrappedUser) {
        allLines.push({ text: line, color: "yellow" });
      }
    } else if (item.type === "assistant") {
      if (item.content.trim()) {
        const lines = item.content.split("\n");
        // Build timing string based on streaming vs non-streaming
        let timingStr: string | null = null;
        if (item.endTime) {
          const totalTime = item.endTime - item.promptTime;
          if (item.firstChunkTime && item.chunkCount > 1) {
            // Streaming: show TTFT and tokens/sec
            const ttft = item.firstChunkTime - item.promptTime;
            const streamDuration = item.endTime - item.firstChunkTime;
            // Rough estimate: ~4 chars per token on average
            const estimatedTokens = Math.round(item.content.length / 4);
            const tokensPerSec = streamDuration > 0 ? (estimatedTokens / (streamDuration / 1000)).toFixed(0) : "—";
            timingStr = `${formatDuration(totalTime)} • ttft ${formatDuration(ttft)} • ${tokensPerSec} tok/s`;
          } else {
            // Non-streaming or single chunk: just show inference time
            timingStr = formatDuration(totalTime);
          }
        }
        for (let j = 0; j < lines.length; j++) {
          // Wrap each line with consistent indentation
          const wrappedLines = wrapText(`  ${lines[j]}`, contentWidth, "  ");
          for (const wrappedLine of wrappedLines) {
            allLines.push({ text: wrappedLine });
          }
          // Add timing annotation after last line
          if (j === lines.length - 1 && timingStr) {
            allLines.push({ text: `  ${timingStr}`, dim: true });
          }
        }
      }
    } else if (item.type === "tool_call") {
      const statusIcon =
        item.status === "completed" ? "✓" :
        item.status === "failed" ? "✗" :
        item.status === "in_progress" ? "◐" : "○";

      const statusColor =
        item.status === "completed" ? "green" :
        item.status === "failed" ? "red" :
        item.status === "in_progress" ? "yellow" : "gray";

      // Calculate execution time
      const execTime = item.resultTime ? formatDuration(item.resultTime - item.callTime) : null;
      
      // Check if this is the last in a group of parallel tool calls
      // (parallel = started before the previous one completed)
      const isLastInGroup = (() => {
        if (i + 1 >= items.length) return true;
        const next = items[i + 1];
        if (next.type !== "tool_call") return true;
        // Next is parallel if it started before this one completed
        if (item.resultTime && next.callTime < item.resultTime) return false;
        return true;
      })();
      
      // Find inference time for this group (time from last completed event to first call in group)
      let inferenceTime: string | null = null;
      if (isLastInGroup && item.resultTime) {
        // Find the first tool call in this parallel group
        let firstCallTime = item.callTime;
        for (let j = i - 1; j >= 0; j--) {
          const prev = items[j];
          if (prev.type === "tool_call") {
            // Check if prev is in the same parallel group (started before any in group completed)
            // Simple heuristic: if this call started before prev completed, they're parallel
            if (prev.resultTime && item.callTime < prev.resultTime) {
              firstCallTime = Math.min(firstCallTime, prev.callTime);
            } else {
              break;
            }
          } else {
            break;
          }
        }
        
        // Find reference time (user message, completed tool, or assistant text before the group)
        let referenceTime: number | null = null;
        for (let j = i; j >= 0; j--) {
          const prev = items[j];
          if (prev.type === "user") {
            referenceTime = prev.timestamp;
            break;
          } else if (prev.type === "tool_call" && prev.resultTime && prev.resultTime <= firstCallTime) {
            referenceTime = prev.resultTime;
            break;
          } else if (prev.type === "assistant" && prev.endTime) {
            referenceTime = prev.endTime;
            break;
          }
        }
        
        if (referenceTime !== null) {
          inferenceTime = formatDuration(firstCallTime - referenceTime);
        }
      }

      // Build the timing suffix: "infer | exec" or "      | exec"
      let timingSuffix = "";
      if (execTime) {
        // Pad inference time to align the pipe
        const inferPart = inferenceTime ? inferenceTime.padStart(6) : "      ";
        timingSuffix = `${inferPart} | ${execTime}`;
      }

      // Calculate padding to right-align timing
      const titlePart = `  ${statusIcon} ${item.title}`;
      const padding = timingSuffix ? Math.max(1, terminalWidth - titlePart.length - timingSuffix.length - 4) : 0;
      const fullLine = timingSuffix 
        ? `${titlePart}${' '.repeat(padding)}${timingSuffix}`
        : titlePart;

      allLines.push({ text: fullLine, color: statusColor, timingSuffix });

      if (item.result) {
        const resultLines = item.result.split("\n");
        const preview = resultLines.length > 6
          ? [...resultLines.slice(0, 5), `... (${resultLines.length - 5} more lines)`]
          : resultLines;
        for (const line of preview) {
          // Wrap result lines with deeper indentation
          const wrappedLines = wrapText(`    ${line}`, contentWidth, "    ");
          for (const wrappedLine of wrappedLines) {
            allLines.push({ text: wrappedLine, dim: true });
          }
        }
      }
    }
  }

  // Add streaming text with live timing
  if (streamingText.trim()) {
    const lines = streamingText.split("\n");
    for (const line of lines) {
      // Wrap streaming text with consistent indentation
      const wrappedLines = wrapText(`  ${line}`, contentWidth, "  ");
      for (const wrappedLine of wrappedLines) {
        allLines.push({ text: wrappedLine });
      }
    }
    // Show live stats: total time, TTFT (if we have it), and live tokens/sec
    if (firstChunkTime && promptSentTime) {
      const totalTime = Date.now() - promptSentTime;
      const ttft = firstChunkTime - promptSentTime;
      const streamDuration = Date.now() - firstChunkTime;
      const estimatedTokens = Math.round(streamingText.length / 4);
      const tokensPerSec = streamDuration > 100 ? (estimatedTokens / (streamDuration / 1000)).toFixed(0) : "—";
      allLines.push({ text: `  ▋ ${formatDuration(totalTime)} • ttft ${formatDuration(ttft)} • ${tokensPerSec} tok/s`, dim: true });
    } else if (promptSentTime) {
      const elapsed = formatDuration(Date.now() - promptSentTime);
      allLines.push({ text: `  ▋ ${elapsed}`, dim: true });
    }
  } else if (promptSentTime) {
    // Show cursor while waiting for first chunk
    const elapsed = formatDuration(Date.now() - promptSentTime);
    allLines.push({ text: `  ▋ waiting ${elapsed}`, dim: true });
  }

  return (
    <Box flexDirection="column" flexGrow={1} paddingX={1}>
      {allLines.map((line, i) => {
        // For tool calls with timing, render title in color and timing dimmed
        if (line.timingSuffix) {
          const titlePart = line.text.slice(0, line.text.length - line.timingSuffix.length);
          return (
            <Box key={i}>
              <Text color={line.color as any}>{titlePart}</Text>
              <Text dimColor>{line.timingSuffix}</Text>
            </Box>
          );
        }
        return (
          <Text key={i} color={line.color as any} dimColor={line.dim}>
            {line.text}
          </Text>
        );
      })}
    </Box>
  );
}

interface PermissionPromptProps {
  request: acp.RequestPermissionRequest;
  onRespond: (response: acp.RequestPermissionResponse) => void;
}

function PermissionPrompt({ request, onRespond }: PermissionPromptProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const options = request.options;

  useInput((input, key) => {
    if (key.upArrow) {
      setSelectedIndex((i) => Math.max(0, i - 1));
    } else if (key.downArrow) {
      setSelectedIndex((i) => Math.min(options.length - 1, i + 1));
    } else if (key.return) {
      const option = options[selectedIndex];
      onRespond({ outcome: { outcome: "selected", optionId: option.optionId } });
    } else if (input === "c" || key.escape) {
      onRespond({ outcome: { outcome: "cancelled" } });
    }
  });

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="yellow" paddingX={1} marginY={1}>
      <Text color="yellow" bold>
        ⚠ Permission Required
      </Text>
      <Text>{request.toolCall.title}</Text>
      <Box flexDirection="column" marginTop={1}>
        {options.map((option, index) => (
          <Box key={option.optionId}>
            <Text color={index === selectedIndex ? "green" : "white"}>
              {index === selectedIndex ? "❯ " : "  "}
              {option.name}
            </Text>
            <Text dimColor> ({option.kind})</Text>
          </Box>
        ))}
      </Box>
      <Box marginTop={1}>
        <Text dimColor>↑/↓ select • Enter confirm • c cancel</Text>
      </Box>
    </Box>
  );
}

interface InputAreaProps {
  onSubmit: (value: string) => void;
  disabled: boolean;
}

function InputArea({ onSubmit, disabled }: InputAreaProps) {
  const [value, setValue] = useState("");

  const handleSubmit = useCallback(
    (v: string) => {
      if (v.trim() && !disabled) {
        onSubmit(v.trim());
        setValue("");
      }
    },
    [onSubmit, disabled]
  );

  return (
    <Box borderStyle="single" borderColor={disabled ? "gray" : "green"} paddingX={1}>
      {disabled ? (
        <Box>
          <Text color="yellow">
            <Spinner type="dots" />
          </Text>
        </Box>
      ) : (
        <Box>
          <Text color="green">❯ </Text>
          <TextInput
            value={value}
            onChange={setValue}
            onSubmit={handleSubmit}
            placeholder="Type your message..."
          />
        </Box>
      )}
    </Box>
  );
}

// ============================================================================
// Main App
// ============================================================================

interface AppProps {
  command: string;
  args: string[];
  agentName: string;
}

function App({ command, args, agentName }: AppProps) {
  const { exit } = useApp();
  const [state, setState] = useState<AppState>({ type: "connecting" });
  const [connection, setConnection] = useState<acp.ClientSideConnection | null>(null);
  const [sessionId, setSessionId] = useState<string | undefined>();
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [promptSentTime, setPromptSentTime] = useState<number | null>(null);
  const [firstChunkTime, setFirstChunkTime] = useState<number | null>(null);
  const [chunkCount, setChunkCount] = useState(0);
  const [, setTick] = useState(0); // Force re-renders for live timing display
  const [cwd] = useState(process.cwd());
  const [stderrOutput, setStderrOutput] = useState<string[]>([]);

  // Timer to update live timing display while waiting/streaming
  useEffect(() => {
    if (promptSentTime === null) return;
    const interval = setInterval(() => {
      setTick((t) => t + 1);
    }, 100); // Update 10x per second for smooth display
    return () => clearInterval(interval);
  }, [promptSentTime]);

  // Track when we last received any update (for inference timing)
  const lastUpdateTimeRef = useRef<number>(Date.now());
  // Track timing in refs so we can access them in the finalization closure
  const promptSentTimeRef = useRef<number | null>(null);
  const firstChunkTimeRef = useRef<number | null>(null);
  const chunkCountRef = useRef<number>(0);

  // Handle Ctrl+C
  useInput((input, key) => {
    if (input === "c" && key.ctrl) {
      exit();
    }
  });

  // Connect to agent on mount
  useEffect(() => {
    const agentProcess = spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
    });

    agentProcess.stderr?.on("data", (data: Buffer) => {
      const lines = data.toString().split("\n").filter(line => line.trim());
      if (lines.length > 0) {
        setStderrOutput(prev => [...prev.slice(-50), ...lines]); // Keep last 50 lines
      }
    });

    // Track if process has exited for error handling
    let processExited = false;
    let exitCode: number | null = null;
    let exitSignal: string | null = null;

    const handleProcessExit = () => {
      if (processExited) return;
      processExited = true;
      
      const exitInfo = exitSignal 
        ? `killed by signal ${exitSignal}` 
        : exitCode !== 0 
          ? `exited with code ${exitCode}` 
          : `exited unexpectedly (code 0)`;
      
      // Use setTimeout to ensure we capture any final stderr
      setTimeout(() => {
        setStderrOutput(prev => {
          const recentStderr = prev.slice(-10).join("\n");
          const errorMsg = recentStderr 
            ? `Agent ${exitInfo}\n\nRecent stderr:\n${recentStderr}`
            : `Agent ${exitInfo}`;
          setState({ type: "error", message: errorMsg });
          return prev;
        });
      }, 100);
    };

    agentProcess.on("error", (err) => {
      processExited = true;
      setState({ type: "error", message: `Failed to start agent: ${err.message}` });
    });

    agentProcess.on("exit", (code, signal) => {
      exitCode = code;
      exitSignal = signal as string | null;
      // Don't immediately show error - wait a moment for close event
      // But if code is non-zero, we know it's an error
      if (code !== 0 || signal) {
        handleProcessExit();
      }
    });

    agentProcess.on("close", (code, signal) => {
      exitCode = code;
      exitSignal = signal as string | null;
      // Process fully closed - if we haven't handled exit yet, do it now
      handleProcessExit();
    });

    // Also detect if stdout closes unexpectedly (process died)
    agentProcess.stdout?.on("close", () => {
      if (!processExited) {
        handleProcessExit();
      }
    });

    // Create streams for the SDK
    const input = Writable.toWeb(agentProcess.stdin!);
    const output = Readable.toWeb(agentProcess.stdout!) as ReadableStream<Uint8Array>;
    const stream = acp.ndJsonStream(input, output);

    // Create client implementation
    const client: acp.Client = {
      async sessionUpdate(notification: acp.SessionNotification): Promise<void> {
        const update = notification.update;
        const now = Date.now();

        switch (update.sessionUpdate) {
          case "agent_message_chunk": {
            const block = update.content;
            if (block.type === "text") {
              // Track first chunk time
              if (firstChunkTimeRef.current === null) {
                setFirstChunkTime(now);
                firstChunkTimeRef.current = now;
              }
              // Increment chunk count
              chunkCountRef.current += 1;
              setChunkCount(chunkCountRef.current);
              
              setStreamingText((prev) => prev + (block.text as string));
            }
            lastUpdateTimeRef.current = now;
            break;
          }

          case "tool_call": {
            // Add new tool call to history
            setHistory((prev) => [
              ...prev,
              {
                type: "tool_call",
                id: update.toolCallId,
                title: update.title || "Tool call",
                status: (update.status as any) || "pending",
                callTime: now,
              },
            ]);
            lastUpdateTimeRef.current = now;
            break;
          }

          case "tool_call_update": {
            // Update existing tool call status and/or result
            setHistory((prev) => {
              const newHistory = [...prev];
              for (let i = newHistory.length - 1; i >= 0; i--) {
                const item = newHistory[i];
                if (item.type === "tool_call" && item.id === update.toolCallId) {
                  // Extract result text from content if present
                  let resultText = item.result;
                  if (update.content) {
                    resultText = (update.content as any[])
                      .filter((c: any) => c.type === "text")
                      .map((c: any) => c.text)
                      .join("\n");
                  }
                  // Mark result time when status becomes completed
                  const isCompleting = update.status === "completed" && item.status !== "completed";
                  newHistory[i] = {
                    ...item,
                    status: update.status || item.status,
                    result: resultText,
                    resultTime: isCompleting ? now : item.resultTime,
                  };
                  break;
                }
              }
              return newHistory;
            });
            lastUpdateTimeRef.current = now;
            break;
          }
        }
      },

      async requestPermission(request: acp.RequestPermissionRequest): Promise<acp.RequestPermissionResponse> {
        return new Promise((resolve) => {
          setState({ type: "permission", request, respond: resolve });
        });
      },
    };

    // Create connection
    const conn = new acp.ClientSideConnection((_agent) => client, stream);

    (async () => {
      try {
        await conn.initialize({
          protocolVersion: acp.PROTOCOL_VERSION,
          clientCapabilities: {},
        });

        const sessionResponse = await conn.newSession({
          cwd,
          mcpServers: [],
        });

        setConnection(conn);
        setSessionId(sessionResponse.sessionId);
        setState({ type: "ready" });
      } catch (err) {
        setState({ type: "error", message: (err as Error).message });
      }
    })();

    return () => {
      agentProcess.kill();
    };
  }, [command, args, cwd]);

  // Handle prompt submission
  const handleSubmit = useCallback(
    async (input: string) => {
      if (!connection || !sessionId) return;

      const now = Date.now();

      // Add user message to history
      setHistory((prev) => [...prev, { type: "user", content: input, timestamp: now }]);
      setStreamingText("");
      setPromptSentTime(now);
      promptSentTimeRef.current = now;
      setFirstChunkTime(null);
      firstChunkTimeRef.current = null;
      setChunkCount(0);
      chunkCountRef.current = 0;
      setState({ type: "prompting" });
      lastUpdateTimeRef.current = now;

      try {
        await connection.prompt({
          sessionId,
          prompt: [{ type: "text", text: input }],
        });

        // Finalize: add any remaining streaming text as assistant message
        const endTime = Date.now();
        const promptTime = promptSentTimeRef.current || lastUpdateTimeRef.current;
        const firstChunk = firstChunkTimeRef.current;
        const chunks = chunkCountRef.current;
        setStreamingText((currentText) => {
          if (currentText.trim()) {
            setHistory((prev) => {
              return [...prev, { 
                type: "assistant", 
                content: currentText, 
                promptTime,
                firstChunkTime: firstChunk ?? undefined,
                endTime,
                chunkCount: chunks,
              }];
            });
          }
          return "";
        });
        setPromptSentTime(null);
        promptSentTimeRef.current = null;
        setFirstChunkTime(null);
        firstChunkTimeRef.current = null;
        setChunkCount(0);
        chunkCountRef.current = 0;

        setState({ type: "ready" });
      } catch (err) {
        setState({ type: "error", message: (err as Error).message });
      }
    },
    [connection, sessionId]
  );

  // Handle permission response
  const handlePermissionResponse = useCallback((response: acp.RequestPermissionResponse) => {
    if (state.type === "permission") {
      state.respond(response);
    }
    setState({ type: "prompting" });
  }, [state]);

  // Derive status for error display
  const errorMessage = state.type === "error" ? state.message : null;

  return (
    <Box flexDirection="column">
      {/* Header */}
      <Box paddingX={1} marginBottom={1}>
        <Text color="cyan" bold>
          ACP Test Client
        </Text>
        <Text dimColor> │ </Text>
        <Text color="green">{agentName}</Text>
        {sessionId && (
          <>
            <Text dimColor> │ </Text>
            <Text dimColor>session:{sessionId.slice(0, 8)}</Text>
          </>
        )}
      </Box>

      {/* Error banner if any */}
      {errorMessage && (
        <Box flexDirection="column" paddingX={1} marginBottom={1}>
          {errorMessage.split("\n").map((line, i) => (
            <Text key={i} color="red">{i === 0 ? `Error: ${line}` : line}</Text>
          ))}
        </Box>
      )}

      {/* Stderr output (collapsible, shows last few lines) */}
      {stderrOutput.length > 0 && state.type !== "error" && (
        <Box flexDirection="column" paddingX={1} marginBottom={1} borderStyle="single" borderColor="yellow">
          <Text color="yellow" bold>stderr ({stderrOutput.length} lines)</Text>
          {stderrOutput.slice(-5).map((line, i) => (
            <Text key={i} dimColor>{line}</Text>
          ))}
        </Box>
      )}

      {/* History */}
      <HistoryView items={history} streamingText={streamingText} promptSentTime={promptSentTime} firstChunkTime={firstChunkTime} chunkCount={chunkCount} />

      {/* Permission prompt (overlays if needed) */}
      {state.type === "permission" && (
        <PermissionPrompt request={state.request} onRespond={handlePermissionResponse} />
      )}

      {/* Input area */}
      <InputArea
        onSubmit={handleSubmit}
        disabled={state.type !== "ready"}
      />
    </Box>
  );
}

// ============================================================================
// Entry point
// ============================================================================

const { config, showHelp } = parseArgs(process.argv);

if (showHelp) {
  printHelp();
  process.exit(0);
}

console.clear();
render(<App command={config.command} args={config.args} agentName={config.name} />, {
  exitOnCtrlC: false,
});
