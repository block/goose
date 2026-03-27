#!/usr/bin/env node
import React, { useState, useEffect, useCallback, useRef } from "react";
import { Box, Text, render, useApp, useInput, useStdout, measureElement } from "ink";
import type { DOMElement } from "ink";
import TextInput from "ink-text-input";
import meow from "meow";
import { spawn } from "node:child_process";
import { Readable, Writable } from "node:stream";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type {
  SessionNotification,
  RequestPermissionRequest,
  RequestPermissionResponse,
  ToolCallContent,
  ToolCallStatus,
  ToolKind,
  Stream,
  ContentChunk,
  ToolCall,
  ToolCallUpdate,
  SessionUpdate,
} from "@agentclientprotocol/sdk";
import { ndJsonStream } from "@agentclientprotocol/sdk";
import { GooseClient } from "@aaif/goose-acp";
import { renderMarkdown } from "./markdown.js";
import { buildToolCallCardLines, ToolCallCompact, findFeaturedToolCallId } from "./toolcall.js";
import type { ToolCallInfo } from "./toolcall.js";
import { CRANBERRY, TEAL, GOLD, TEXT_PRIMARY, TEXT_SECONDARY, TEXT_DIM, RULE_COLOR } from "./colors.js";

interface PendingPermission {
  toolTitle: string;
  options: Array<{ optionId: string; name: string; kind: string }>;
  resolve: (response: RequestPermissionResponse) => void;
}

type ResponseItem =
  | (ContentChunk & { itemType: "content_chunk" })
  | (ToolCall & { itemType: "tool_call" });

interface Turn {
  userText: string;
  responseItems: ResponseItem[];
  toolCallsById: Map<string, number>; // maps toolCallId to index in responseItems
}

function isErrorStatus(status: string): boolean {
  return status.startsWith("error") || status.startsWith("failed");
}

const GOOSE_FRAMES = [
  [
    "    ,_",
    "   (o >",
    "   //\\",
    "   \\\\ \\",
    "    \\\\_/",
    "     |  |",
    "     ^ ^",
  ],
  [
    "     ,_",
    "    (o >",
    "    //\\",
    "    \\\\ \\",
    "     \\\\_/",
    "    /  |",
    "   ^   ^",
  ],
  [
    "    ,_",
    "   (o >",
    "   //\\",
    "   \\\\ \\",
    "    \\\\_/",
    "     |  |",
    "     ^  ^",
  ],
  [
    "   ,_",
    "  (o >",
    "  //\\",
    "  \\\\ \\",
    "   \\\\_/",
    "    |  \\",
    "    ^   ^",
  ],
];

const GREETING_MESSAGES = [
  "What would you like to work on?",
  "Ready to build something amazing?",
  "What would you like to explore?",
  "What's on your mind?",
  "What shall we create today?",
  "What project needs attention?",
  "What would you like to tackle?",
  "What needs to be done?",
  "What's the plan for today?",
  "Ready to create something great?",
  "What can be built today?",
  "What's the next challenge?",
  "What progress can be made?",
  "What would you like to accomplish?",
  "What task awaits?",
  "What's the mission today?",
  "What can be achieved?",
  "What project is ready to begin?",
];

const INITIAL_GREETING =
  GREETING_MESSAGES[Math.floor(Math.random() * GREETING_MESSAGES.length)]!;

const SPINNER_FRAMES = ["◐", "◓", "◑", "◒"];

const PERMISSION_LABELS: Record<string, string> = {
  allow_once: "Allow once",
  allow_always: "Always allow",
  reject_once: "Reject once",
  reject_always: "Always reject",
};

const PERMISSION_KEYS: Record<string, string> = {
  allow_once: "y",
  allow_always: "a",
  reject_once: "n",
  reject_always: "N",
};

const INDENT = 3;
const CONTENT_INDENT = 5;

function Rule({ width }: { width: number }) {
  return <Text color={RULE_COLOR}>{"─".repeat(Math.max(width, 1))}</Text>;
}

function Spinner({ idx }: { idx: number }) {
  return (
    <Text color={CRANBERRY}>
      {SPINNER_FRAMES[idx % SPINNER_FRAMES.length]}
    </Text>
  );
}

function Header({
  width,
  status,
  loading,
  spinIdx,
  hasPendingPermission,
  turnInfo,
}: {
  width: number;
  status: string;
  loading: boolean;
  spinIdx: number;
  hasPendingPermission: boolean;
  turnInfo?: { current: number; total: number };
}) {
  const statusColor = status === "ready" ? TEAL : isErrorStatus(status) ? CRANBERRY : TEXT_DIM;

  return (
    <Box flexDirection="column" width={width}>
      <Box justifyContent="space-between" width={width}>
        <Box>
          <Text color={TEXT_PRIMARY} bold>
            goose
          </Text>
          <Text color={RULE_COLOR}> · </Text>
          <Text color={statusColor}>{status}</Text>
          {loading && !hasPendingPermission && (
            <Text>
              {" "}
              <Spinner idx={spinIdx} />
            </Text>
          )}
        </Box>
        <Box>
          {turnInfo && turnInfo.total > 1 && (
            <Text color={TEXT_DIM}>
              {turnInfo.current}/{turnInfo.total}
              {"  "}
            </Text>
          )}
          <Text color={TEXT_DIM}>^C exit</Text>
        </Box>
      </Box>
      <Rule width={width} />
    </Box>
  );
}

function UserPrompt({ text }: { text: string }) {
  return (
    <Box paddingLeft={INDENT} paddingTop={1}>
      <Text color={CRANBERRY} bold>
        {"❯ "}
      </Text>
      <Text color={TEXT_PRIMARY} bold>
        {text}
      </Text>
    </Box>
  );
}

function PermissionDialog({
  toolTitle,
  options,
  selectedIdx,
  width,
}: {
  toolTitle: string;
  options: Array<{ optionId: string; name: string; kind: string }>;
  selectedIdx: number;
  width: number;
}) {
  const dialogWidth = Math.min(width - CONTENT_INDENT - 2, 58);
  return (
    <Box
      flexDirection="column"
      marginLeft={CONTENT_INDENT}
      marginTop={1}
      paddingX={2}
      paddingY={1}
      borderStyle="round"
      borderColor={GOLD}
      width={dialogWidth}
    >
      <Text color={GOLD} bold>
        🔒 Permission required
      </Text>
      <Box marginTop={1}>
        <Text color={TEXT_PRIMARY}>{toolTitle}</Text>
      </Box>
      <Box marginTop={1} flexDirection="column">
        {options.map((opt, i) => {
          const key = PERMISSION_KEYS[opt.kind] ?? String(i + 1);
          const label = PERMISSION_LABELS[opt.kind] ?? opt.name;
          const active = i === selectedIdx;
          return (
            <Box key={opt.optionId}>
              <Text color={active ? GOLD : RULE_COLOR}>
                {active ? " ▸ " : "   "}
              </Text>
              <Text
                color={active ? TEXT_PRIMARY : TEXT_SECONDARY}
                bold={active}
              >
                [{key}] {label}
              </Text>
            </Box>
          );
        })}
      </Box>
      <Box marginTop={1}>
        <Text color={TEXT_DIM}>↑↓ select · enter confirm · esc cancel</Text>
      </Box>
    </Box>
  );
}

function QueuedMessage({ text }: { text: string }) {
  return (
    <Box paddingLeft={INDENT}>
      <Text color={TEXT_DIM}>❯ </Text>
      <Text color={TEXT_DIM}>{text}</Text>
      <Text color={GOLD} dimColor>
        {" "}
        (queued)
      </Text>
    </Box>
  );
}

function InputBar({
  width,
  input,
  onChange,
  onSubmit,
  queued,
  scrollHint,
}: {
  width: number;
  input: string;
  onChange: (v: string) => void;
  onSubmit: (v: string) => void;
  queued: boolean;
  scrollHint: boolean;
}) {
  return (
    <Box flexDirection="column" width={width} marginBottom={1}>
      <Box
        flexDirection="column"
        borderStyle="round"
        borderColor={RULE_COLOR}
        paddingX={1}
        width={width}
      >
        <Box justifyContent="space-between">
          <Box flexGrow={1}>
            <Text color={CRANBERRY} bold>
              {"❯ "}
            </Text>
            <TextInput value={input} onChange={onChange} onSubmit={onSubmit} />
          </Box>
          {scrollHint && <Text color={TEXT_DIM}>shift+↑↓ history</Text>}
        </Box>
        {queued && (
          <Box>
            <Text color={GOLD} dimColor italic>
              message queued — will send when goose finishes
            </Text>
          </Box>
        )}
      </Box>
    </Box>
  );
}

function buildTurnBodyLines({
  turn,
  width,
  loading,
  status,
  spinIdx,
  pendingPermission,
  permissionIdx,
  toolCallsExpanded,
}: {
  turn: Turn;
  width: number;
  loading: boolean;
  status: string;
  spinIdx: number;
  pendingPermission: PendingPermission | null;
  permissionIdx: number;
  toolCallsExpanded: boolean;
}): React.ReactNode[] {
  const lines: React.ReactNode[] = [];

  lines.push(<Box key="gap-top" height={1} />);

  let toolCallIndex = 0;
  let textChunkIndex = 0;

  // Render items in the order they arrived
  for (let i = 0; i < turn.responseItems.length; i++) {
    const item = turn.responseItems[i]!;

    if (item.itemType === "tool_call") {
      const tcId = item.toolCallId;
      const info: ToolCallInfo = {
        toolCallId: item.toolCallId,
        title: item.title,
        status: item.status ?? "pending",
        kind: item.kind,
        rawInput: item.rawInput,
        rawOutput: item.rawOutput,
        content: item.content,
        locations: item.locations,
      };

      if (toolCallsExpanded) {
        const cardLines = buildToolCallCardLines(info, CONTENT_INDENT, width, true, `tc-${tcId}`);
        lines.push(...cardLines);
      } else {
        const compactLines = ToolCallCompact({
          info,
          indent: CONTENT_INDENT,
          width,
          keyPrefix: `tc-${tcId}`,
          showTabHint: toolCallIndex === 0,
        });
        lines.push(...compactLines);
      }
      toolCallIndex++;
    } else if (item.itemType === "content_chunk") {
      if (item.content.type === "text") {
        const text = item.content.text;
        if (text) {
          const rendered = renderMarkdown(text);
          const mdLines = rendered.split("\n");
          for (let j = 0; j < mdLines.length; j++) {
            lines.push(
              <Box key={`text-${textChunkIndex}-${j}`} paddingLeft={CONTENT_INDENT}>
                <Text>{mdLines[j]}</Text>
              </Box>,
            );
          }
          textChunkIndex++;
        }
      }
    }
  }

  if (loading && !pendingPermission) {
    lines.push(
      <Box key="loading" paddingLeft={CONTENT_INDENT}>
        <Spinner idx={spinIdx} />
        <Text color={TEXT_DIM} italic>
          {" "}
          {status}
        </Text>
      </Box>,
    );
  }

  if (pendingPermission) {
    lines.push(
      <PermissionDialog
        key="permission"
        toolTitle={pendingPermission.toolTitle}
        options={pendingPermission.options}
        selectedIdx={permissionIdx}
        width={width}
      />,
    );
  }

  return lines;
}

function ScrollableBody({
  lines,
  width,
  scrollOffset,
}: {
  lines: React.ReactNode[];
  width: number;
  scrollOffset: number;
}) {
  const ref = useRef<DOMElement>(null);
  const [measured, setMeasured] = useState(0);

  useEffect(() => {
    if (ref.current) {
      const { height } = measureElement(ref.current);
      if (height !== measured) setMeasured(height);
    }
  });

  const total = lines.length;
  const availableHeight = measured || total;
  const needsScroll = total > availableHeight;
  const viewSize = needsScroll
    ? Math.max(availableHeight - 2, 1)
    : availableHeight;
  const maxOffset = Math.max(total - viewSize, 0);
  const clampedOffset = Math.min(Math.max(scrollOffset, 0), maxOffset);
  const endIdx = total - clampedOffset;
  const startIdx = Math.max(endIdx - viewSize, 0);
  const visible = lines.slice(startIdx, endIdx);

  const hiddenAbove = startIdx;
  const hiddenBelow = Math.max(total - endIdx, 0);

  return (
    <Box ref={ref} flexDirection="column" flexGrow={1}>
      {needsScroll && (
        <Box justifyContent="center" width={width} height={1}>
          {hiddenAbove > 0 ? (
            <Text color={TEXT_DIM}>▲ {hiddenAbove} more (↑)</Text>
          ) : (
            <Text> </Text>
          )}
        </Box>
      )}
      <Box flexDirection="column" flexGrow={1} overflowY="hidden">
        {visible}
      </Box>
      {needsScroll && (
        <Box justifyContent="center" width={width} height={1}>
          {hiddenBelow > 0 ? (
            <Text color={TEXT_DIM}>▼ {hiddenBelow} more (↓)</Text>
          ) : (
            <Text> </Text>
          )}
        </Box>
      )}
    </Box>
  );
}

function SplashScreen({
  animFrame,
  width,
  height,
  status,
  loading,
  spinIdx,
  showInput,
  input,
  onInputChange,
  onInputSubmit,
}: {
  animFrame: number;
  width: number;
  height: number;
  status: string;
  loading: boolean;
  spinIdx: number;
  showInput: boolean;
  input: string;
  onInputChange: (v: string) => void;
  onInputSubmit: (v: string) => void;
}) {
  const frame = GOOSE_FRAMES[animFrame % GOOSE_FRAMES.length]!;
  const statusColor = status === "ready" ? TEAL : isErrorStatus(status) ? CRANBERRY : TEXT_DIM;
  const inputWidth = Math.min(56, width - 8);

  return (
    <Box
      flexDirection="column"
      alignItems="center"
      justifyContent="center"
      width={width}
      height={height}
    >
      <Box flexDirection="column" alignItems="center">
        {frame.map((line, i) => (
          <Text key={i} color={TEXT_PRIMARY}>
            {line}
          </Text>
        ))}
      </Box>

      <Box marginTop={1}>
        <Text color={TEXT_PRIMARY} bold>
          goose
        </Text>
      </Box>
      <Text color={TEXT_DIM}>your on-machine AI agent</Text>

      {showInput ? (
        <Box flexDirection="column" alignItems="center" marginTop={2}>
          <Box width={inputWidth}>
            <Rule width={inputWidth} />
          </Box>
          <Box>
            <Text color={CRANBERRY} bold>
              {"❯ "}
            </Text>
            <TextInput
              value={input}
              placeholder={INITIAL_GREETING}
              onChange={onInputChange}
              onSubmit={onInputSubmit}
              showCursor
            />
          </Box>
          <Box width={inputWidth}>
            <Rule width={inputWidth} />
          </Box>
        </Box>
      ) : (
        <Box marginTop={2} gap={1}>
          {loading && <Spinner idx={spinIdx} />}
          <Text color={statusColor}>{status}</Text>
        </Box>
      )}
    </Box>
  );
}

function App({
  serverConnection,
  initialPrompt,
}: {
  serverConnection: Stream | string;
  initialPrompt?: string;
}) {
  const { exit } = useApp();
  const { stdout } = useStdout();
  const termWidth = stdout?.columns ?? 80;
  const termHeight = stdout?.rows ?? 24;

  const [turns, setTurns] = useState<Turn[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState("connecting…");
  const [spinIdx, setSpinIdx] = useState(0);
  const [gooseFrame, setGooseFrame] = useState(0);
  const [bannerVisible, setBannerVisible] = useState(true);
  const [pendingPermission, setPendingPermission] =
    useState<PendingPermission | null>(null);
  const [permissionIdx, setPermissionIdx] = useState(0);
  const [queuedMessages, setQueuedMessages] = useState<string[]>([]);

  const [viewTurnIdx, setViewTurnIdx] = useState(-1);
  const [toolCallsExpanded, setToolCallsExpanded] = useState(false);
  const [scrollOffset, setScrollOffset] = useState(0);

  const clientRef = useRef<GooseClient | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const streamBuf = useRef("");
  const sentInitialPrompt = useRef(false);
  const queueRef = useRef<string[]>([]);
  const isProcessingRef = useRef(false);

  useEffect(() => {
    const t = setInterval(() => {
      setSpinIdx((i) => (i + 1) % SPINNER_FRAMES.length);
      setGooseFrame((f) => f + 1);
    }, 300);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    if (turns.length > 0) setBannerVisible(false);
  }, [turns]);

  useEffect(() => {
    setToolCallsExpanded(false);
    setScrollOffset(0);
  }, [viewTurnIdx, turns.length]);

  const appendAgent = useCallback((text: string) => {
    setTurns((prev) => {
      if (prev.length === 0) return prev;
      const last = { ...prev[prev.length - 1]! };
      const newItems = [...last.responseItems];
      
      // If last item is a content chunk with text, append to it; otherwise create new content chunk
      if (newItems.length > 0 && newItems[newItems.length - 1]!.itemType === "content_chunk") {
        const lastItem = newItems[newItems.length - 1] as ContentChunk & { itemType: "content_chunk" };
        if (lastItem.content.type === "text") {
          newItems[newItems.length - 1] = {
            ...lastItem,
            content: {
              ...lastItem.content,
              text: lastItem.content.text + text,
            },
          };
        } else {
          // Last item is not text, create new content chunk
          newItems.push({
            itemType: "content_chunk",
            content: { type: "text", text },
          });
        }
      } else {
        // No items or last item is tool call, create new content chunk
        newItems.push({
          itemType: "content_chunk",
          content: { type: "text", text },
        });
      }
      
      return [...prev.slice(0, -1), { ...last, responseItems: newItems }];
    });
  }, []);

  const handleToolCall = useCallback(
    (tc: ToolCall) => {
      setTurns((prev) => {
        if (prev.length === 0) return prev;
        const last = { ...prev[prev.length - 1]! };
        
        const newItems = [...last.responseItems];
        const newById = new Map(last.toolCallsById);
        
        // Add new tool call to the array
        const index = newItems.length;
        newItems.push({ ...tc, itemType: "tool_call" });
        newById.set(tc.toolCallId, index);
        
        return [
          ...prev.slice(0, -1),
          { ...last, responseItems: newItems, toolCallsById: newById },
        ];
      });
    },
    [],
  );

  const handleToolCallUpdate = useCallback(
    (update: ToolCallUpdate) => {
      setTurns((prev) => {
        if (prev.length === 0) return prev;
        const last = { ...prev[prev.length - 1]! };
        
        const index = last.toolCallsById.get(update.toolCallId);
        if (index === undefined) return prev;
        
        const item = last.responseItems[index];
        if (!item || item.itemType !== "tool_call") return prev;
        
        const updated: ToolCall & { itemType: "tool_call" } = { ...item };
        if (update.title != null) updated.title = update.title;
        if (update.status != null) updated.status = update.status;
        if (update.kind != null) updated.kind = update.kind;
        if (update.rawInput !== undefined) updated.rawInput = update.rawInput;
        if (update.rawOutput !== undefined) updated.rawOutput = update.rawOutput;
        if (update.content != null) updated.content = update.content;
        if (update.locations != null) updated.locations = update.locations;
        
        const newItems = [...last.responseItems];
        newItems[index] = updated;
        
        return [...prev.slice(0, -1), { ...last, responseItems: newItems }];
      });
    },
    [],
  );

  const addUserTurn = useCallback((text: string) => {
    setTurns((prev) => [
      ...prev,
      {
        userText: text,
        responseItems: [],
        toolCallsById: new Map(),
      },
    ]);
    setViewTurnIdx(-1);
    setToolCallsExpanded(false);
    setScrollOffset(0);
  }, []);

  const resolvePermission = useCallback(
    (option: { optionId: string } | "cancelled") => {
      if (!pendingPermission) return;
      const { resolve } = pendingPermission;
      if (option === "cancelled") {
        resolve({ outcome: { outcome: "cancelled" } });
      } else {
        resolve({
          outcome: { outcome: "selected", optionId: option.optionId },
        });
      }
      setPendingPermission(null);
      setPermissionIdx(0);
    },
    [pendingPermission],
  );

  const executePrompt = useCallback(
    async (text: string) => {
      const client = clientRef.current;
      const sid = sessionIdRef.current;
      if (!client || !sid) return;

      addUserTurn(text);
      setLoading(true);
      setStatus("thinking…");
      streamBuf.current = "";

      try {
        const result = await client.prompt({
          sessionId: sid,
          prompt: [{ type: "text", text }],
        });

        if (streamBuf.current) appendAgent("");

        setStatus(
          result.stopReason === "end_turn"
            ? "ready"
            : `stopped: ${result.stopReason}`,
        );
      } catch (e: unknown) {
        const errMsg = e instanceof Error ? e.message : String(e);
        setStatus(`error: ${errMsg}`);
      } finally {
        setLoading(false);
      }
    },
    [appendAgent, addUserTurn],
  );

  const processQueue = useCallback(async () => {
    if (isProcessingRef.current) return;
    isProcessingRef.current = true;

    while (queueRef.current.length > 0) {
      const next = queueRef.current.shift()!;
      setQueuedMessages([...queueRef.current]);
      await executePrompt(next);
    }

    isProcessingRef.current = false;
  }, [executePrompt]);

  const sendPrompt = useCallback(
    async (text: string) => {
      await executePrompt(text);
      if (queueRef.current.length > 0) processQueue();
    },
    [executePrompt, processQueue],
  );

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        setStatus("initializing…");

        const client = new GooseClient(
          () => ({
            sessionUpdate: async (params: SessionNotification) => {
              const update = params.update;

              if (update.sessionUpdate === "agent_message_chunk") {
                if (update.content.type === "text") {
                  streamBuf.current += update.content.text;
                  appendAgent(update.content.text);
                }
              } else if (update.sessionUpdate === "tool_call") {
                handleToolCall(update);
              } else if (update.sessionUpdate === "tool_call_update") {
                handleToolCallUpdate(update);
              }
            },
            requestPermission: async (
              params: RequestPermissionRequest,
            ): Promise<RequestPermissionResponse> => {
              return new Promise<RequestPermissionResponse>((resolve) => {
                const toolTitle = params.toolCall.title ?? "unknown tool";
                const options = params.options.map((opt) => ({
                  optionId: opt.optionId,
                  name: opt.name,
                  kind: opt.kind,
                }));
                setPendingPermission({ toolTitle, options, resolve });
                setPermissionIdx(0);
              });
            },
          }),
          serverConnection,
        );

        if (cancelled) return;
        clientRef.current = client;

        setStatus("handshaking…");
        await client.initialize({
          protocolVersion: 0,
          clientInfo: { name: "goose-text", version: "0.1.0" },
          clientCapabilities: {},
        });

        if (cancelled) return;

        setStatus("creating session…");
        const session = await client.newSession({
          cwd: process.cwd(),
          mcpServers: [],
        });

        if (cancelled) return;
        sessionIdRef.current = session.sessionId;
        setLoading(false);
        setStatus("ready");

        if (initialPrompt && !sentInitialPrompt.current) {
          sentInitialPrompt.current = true;
          await sendPrompt(initialPrompt);
          setTimeout(() => exit(), 100);
        }
      } catch (e: unknown) {
        if (cancelled) return;
        const errMsg = e instanceof Error ? e.message : String(e);
        setStatus(`failed: ${errMsg}`);
        setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [
    serverConnection,
    initialPrompt,
    sendPrompt,
    appendAgent,
    handleToolCall,
    handleToolCallUpdate,
    exit,
  ]);

  const handleSubmit = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (!trimmed) return;
      setInput("");
      setViewTurnIdx(-1);
      setToolCallsExpanded(false);
      setScrollOffset(0);

      if (loading || isProcessingRef.current) {
        queueRef.current.push(trimmed);
        setQueuedMessages([...queueRef.current]);
      } else {
        sendPrompt(trimmed);
      }
    },
    [loading, sendPrompt],
  );

  useInput((ch, key) => {
    if (key.escape || (ch === "c" && key.ctrl)) {
      if (pendingPermission) {
        resolvePermission("cancelled");
        return;
      }
      exit();
    }

    if (pendingPermission) {
      const opts = pendingPermission.options;

      if (key.upArrow) {
        setPermissionIdx((i) => (i - 1 + opts.length) % opts.length);
        return;
      }
      if (key.downArrow) {
        setPermissionIdx((i) => (i + 1) % opts.length);
        return;
      }
      if (key.return) {
        const selected = opts[permissionIdx];
        if (selected) resolvePermission({ optionId: selected.optionId });
        return;
      }

      const keyMap: Record<string, string> = {
        y: "allow_once",
        a: "allow_always",
        n: "reject_once",
        N: "reject_always",
      };
      const targetKind = keyMap[ch];
      if (targetKind) {
        const match = opts.find((o) => o.kind === targetKind);
        if (match) resolvePermission({ optionId: match.optionId });
      }
      return;
    }

    if (key.tab) {
      const effectiveIdx =
        viewTurnIdx === -1 ? turns.length - 1 : viewTurnIdx;
      const currentTurn = turns[effectiveIdx];
      if (!currentTurn) return;
      
      // Check if there are any tool calls in the response items
      const hasToolCalls = currentTurn.responseItems.some(item => item.itemType === "tool_call");
      if (!hasToolCalls) return;

      setToolCallsExpanded((prev) => !prev);
      return;
    }

    if (key.upArrow && !key.shift && !key.meta) {
      setScrollOffset((prev) => prev + 3);
      return;
    }
    if (key.downArrow && !key.shift && !key.meta) {
      setScrollOffset((prev) => Math.max(prev - 3, 0));
      return;
    }

    if (key.upArrow && key.shift) {
      setTurns((currentTurns) => {
        if (currentTurns.length <= 1) return currentTurns;
        setViewTurnIdx((prev) => {
          const effectiveIdx =
            prev === -1 ? currentTurns.length - 1 : prev;
          return Math.max(effectiveIdx - 1, 0);
        });
        return currentTurns;
      });
      return;
    }
    if (key.downArrow && key.shift) {
      setTurns((currentTurns) => {
        if (currentTurns.length <= 1) return currentTurns;
        setViewTurnIdx((prev) => {
          if (prev === -1) return -1;
          const next = prev + 1;
          return next >= currentTurns.length ? -1 : next;
        });
        return currentTurns;
      });
      return;
    }
  });

  const GUTTER = 2;
  const innerWidth = Math.max(termWidth - GUTTER * 2, 20);

  if (bannerVisible) {
    return (
      <Box flexDirection="column" width={termWidth} height={termHeight}>
        <SplashScreen
          animFrame={gooseFrame}
          width={termWidth}
          height={termHeight}
          status={status}
          loading={loading}
          spinIdx={spinIdx}
          showInput={!loading && !initialPrompt}
          input={input}
          onInputChange={setInput}
          onInputSubmit={handleSubmit}
        />
      </Box>
    );
  }

  const effectiveTurnIdx =
    viewTurnIdx === -1 ? turns.length - 1 : viewTurnIdx;
  const currentTurn = turns[effectiveTurnIdx];
  const isViewingHistory =
    viewTurnIdx !== -1 && viewTurnIdx < turns.length - 1;
  const isLatest = !isViewingHistory;

  const emptyTurn: Turn = {
    userText: "",
    responseItems: [],
    toolCallsById: new Map(),
  };

  const bodyLines = buildTurnBodyLines({
    turn: currentTurn ?? emptyTurn,
    width: innerWidth,
    loading: isLatest && loading,
    status,
    spinIdx,
    pendingPermission: isLatest ? pendingPermission : null,
    permissionIdx,
    toolCallsExpanded,
  });

  const allBodyLines = isLatest
    ? [
        ...bodyLines,
        ...queuedMessages.map((text, i) => (
          <QueuedMessage key={`q-${i}`} text={text} />
        )),
      ]
    : bodyLines;

  return (
    <Box
      flexDirection="column"
      width={termWidth}
      height={termHeight}
      paddingX={GUTTER}
    >
      <Header
        width={innerWidth}
        status={status}
        loading={loading}
        spinIdx={spinIdx}
        hasPendingPermission={!!pendingPermission}
        turnInfo={
          turns.length > 1
            ? { current: effectiveTurnIdx + 1, total: turns.length }
            : undefined
        }
      />

      {currentTurn ? (
        <>
          <UserPrompt text={currentTurn.userText} />

          <ScrollableBody
            lines={allBodyLines}
            width={innerWidth}
            scrollOffset={scrollOffset}
          />
        </>
      ) : (
        <Box flexDirection="column" flexGrow={1} />
      )}

      {isViewingHistory && (
        <Box flexDirection="column" width={innerWidth}>
          <Rule width={innerWidth} />
          <Box justifyContent="center" width={innerWidth}>
            <Text color={GOLD}>
              turn {effectiveTurnIdx + 1}/{turns.length}
            </Text>
            <Text color={TEXT_DIM}> — shift+↓ to return</Text>
          </Box>
        </Box>
      )}

      {!isViewingHistory && !pendingPermission && !initialPrompt && (
        <InputBar
          width={innerWidth}
          input={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          queued={queuedMessages.length > 0}
          scrollHint={turns.length > 1}
        />
      )}
    </Box>
  );
}

const cli = meow(
  `
  Usage
    $ goose

  Options
    --server, -s  Server URL (default: auto-launch bundled server)
    --text, -t    Send a single prompt and exit
`,
  {
    importMeta: import.meta,
    flags: {
      server: { type: "string", shortFlag: "s" },
      text: { type: "string", shortFlag: "t" },
    },
  },
);

function findServerBinary(): string | null {
  const __dirname = dirname(fileURLToPath(import.meta.url));

  const candidates = [
    join(__dirname, "..", "server-binary.json"),
    join(__dirname, "server-binary.json"),
  ];

  for (const candidate of candidates) {
    try {
      const data = JSON.parse(readFileSync(candidate, "utf-8"));
      return data.binaryPath ?? null;
    } catch {
      // not found here, try next
    }
  }

  return null;
}

let serverProcess: ReturnType<typeof spawn> | null = null;

async function main() {
  let serverConnection: Stream | string;

  if (cli.flags.server) {
    serverConnection = cli.flags.server;
  } else {
    const binary = findServerBinary();
    if (!binary) {
      console.error(
        "No goose binary found. Use --server <url> or install the native package.",
      );
      process.exit(1);
    }

    serverProcess = spawn(binary, ["acp"], {
      stdio: ["pipe", "pipe", "ignore"],
      detached: false,
    });

    serverProcess.on("error", (err) => {
      console.error(`Failed to start goose acp: ${err.message}`);
      process.exit(1);
    });

    const output = Writable.toWeb(serverProcess.stdin!) as WritableStream<Uint8Array>;
    const input = Readable.toWeb(serverProcess.stdout!) as ReadableStream<Uint8Array>;
    serverConnection = ndJsonStream(output, input);
  }

  const { waitUntilExit } = render(
    <App serverConnection={serverConnection} initialPrompt={cli.flags.text} />,
  );

  await waitUntilExit();
  cleanup();
}

function cleanup() {
  if (serverProcess && !serverProcess.killed) {
    serverProcess.kill();
  }
}

process.on("exit", cleanup);
process.on("SIGINT", () => {
  cleanup();
  process.exit(0);
});
process.on("SIGTERM", () => {
  cleanup();
  process.exit(0);
});

main().catch((err) => {
  console.error(err);
  cleanup();
  process.exit(1);
});

