import React, { useState, useEffect, useCallback, useRef } from "react";
import { Box, Text, useApp, useInput, useStdout } from "ink";
import TextInput from "ink-text-input";
import type {
  SessionNotification,
  RequestPermissionRequest,
  RequestPermissionResponse,
} from "@agentclientprotocol/sdk";
import { GooseClient } from "@goose/acp";
import { createHttpStream } from "./transport.js";

interface PendingPermission {
  toolTitle: string;
  options: Array<{ optionId: string; name: string; kind: string }>;
  resolve: (response: RequestPermissionResponse) => void;
}

const CRANBERRY = "#C0354A";
const TEAL = "#3A7D7B";
const GOLD = "#C4883A";
const CEDAR = "#6B5344";

const TEXT_PRIMARY = "#E8E4DF";
const TEXT_SECONDARY = "#8FA4BD";
const TEXT_DIM = "#5A6D84";
const RULE_COLOR = "#2E3D54";

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

interface Turn {
  userText: string;
  toolCalls: string[];
  agentText: string;
}

// ─── Layout constants ───────────────────────────────────────────────────────
//
// Every element indents by a multiple of INDENT (3 spaces). This keeps the
// left edge of user prompts, agent prose, and tool-call badges on a
// predictable grid so the eye can scan vertically without friction.
//
//   col 0   rule / header
//   col 3   user prompt caret + text, input caret + text
//   col 5   agent prose, tool badges, loading spinner, permission dialog

const INDENT = 3;
const CONTENT_INDENT = 5;
const MAX_PROSE_WIDTH = 76;

function Rule({ width }: { width: number }) {
  return (
    <Text color={RULE_COLOR}>{"─".repeat(Math.max(width, 1))}</Text>
  );
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
  const isError =
    status.startsWith("error") || status.startsWith("failed");
  const statusColor = status === "ready" ? TEAL : isError ? CRANBERRY : TEXT_DIM;

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
            <Text> <Spinner idx={spinIdx} /></Text>
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

function ToolBadge({ title, width }: { title: string; width: number }) {
  const badgeWidth = Math.min(width - CONTENT_INDENT - 2, 68);
  return (
    <Box
      marginLeft={CONTENT_INDENT}
      paddingX={1}
      borderStyle="round"
      borderColor={CEDAR}
      borderDimColor
      width={badgeWidth}
    >
      <Text color={TEAL}>⚙ </Text>
      <Text color={TEXT_SECONDARY} italic>
        {title}
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

function inputBarHeight(input: string, width: number, queued: boolean): number {
  // Inner text width: width minus border (2), paddingX (2), prompt "❯ " (2)
  const textWidth = Math.max(width - 2 - 2 - 2, 1);
  // +1 for the trailing cursor character
  const contentLen = input.length + 1;
  const wrappedLines = Math.max(Math.ceil(contentLen / textWidth), 1);
  const queuedLine = queued ? 1 : 0;
  return 2 + wrappedLines + queuedLine + 1;
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
          {scrollHint && (
            <Text color={TEXT_DIM}>shift+↑↓ history</Text>
          )}
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

function wrapText(text: string, width: number): string[] {
  if (width <= 0) return [text];
  const result: string[] = [];
  for (const rawLine of text.split("\n")) {
    if (rawLine.length === 0) {
      result.push("");
      continue;
    }
    let remaining = rawLine;
    while (remaining.length > width) {
      let breakAt = remaining.lastIndexOf(" ", width);
      if (breakAt <= 0) breakAt = width;
      result.push(remaining.slice(0, breakAt));
      remaining = remaining.slice(breakAt).replace(/^ /, "");
    }
    result.push(remaining);
  }
  return result;
}

function TurnResponseBody({
  turn,
  width,
  height,
  scrollOffset,
  loading,
  status,
  spinIdx,
  pendingPermission,
  permissionIdx,
}: {
  turn: Turn;
  width: number;
  height: number;
  scrollOffset: number;
  loading: boolean;
  status: string;
  spinIdx: number;
  pendingPermission: PendingPermission | null;
  permissionIdx: number;
}) {
  const allLines: React.ReactNode[] = [];
  const proseWidth = Math.min(width - CONTENT_INDENT - 1, MAX_PROSE_WIDTH);

  // blank line between user prompt and response content
  allLines.push(<Box key="gap-top" height={1} />);

  for (const tc of turn.toolCalls) {
    allLines.push(
      <ToolBadge key={`tc-${allLines.length}`} title={tc} width={width} />,
    );
  }

  if (turn.agentText) {
    // visual break between tool calls and prose
    if (turn.toolCalls.length > 0) {
      allLines.push(<Box key="gap-tools" height={1} />);
    }
    const wrapped = wrapText(turn.agentText, proseWidth);
    for (const line of wrapped) {
      allLines.push(
        <Box key={`al-${allLines.length}`} paddingLeft={CONTENT_INDENT}>
          <Text color={TEXT_PRIMARY}>{line}</Text>
        </Box>,
      );
    }
  }

  if (loading && !pendingPermission) {
    allLines.push(
      <Box key={`load-${allLines.length}`} paddingLeft={CONTENT_INDENT} marginTop={turn.agentText ? 0 : 0}>
        <Spinner idx={spinIdx} />
        <Text color={TEXT_DIM} italic>
          {" "}
          {status}
        </Text>
      </Box>,
    );
  }

  if (pendingPermission) {
    allLines.push(
      <PermissionDialog
        key={`perm-${allLines.length}`}
        toolTitle={pendingPermission.toolTitle}
        options={pendingPermission.options}
        selectedIdx={permissionIdx}
        width={width}
      />,
    );
  }

  const totalLines = allLines.length;
  const visibleCount = Math.max(height, 1);
  const maxOffset = Math.max(totalLines - visibleCount, 0);
  const offset = Math.min(Math.max(scrollOffset, 0), maxOffset);
  const visible = allLines.slice(offset, offset + visibleCount);
  const hasAbove = offset > 0;
  const hasBelow = offset + visibleCount < totalLines;

  return (
    <Box flexDirection="column" height={height} overflowY="hidden">
      {hasAbove && (
        <Box justifyContent="center" width={width}>
          <Text color={TEXT_DIM}>▲ more</Text>
        </Box>
      )}
      {visible}
      {hasBelow && !hasAbove && visible.length > 1 && (
        <Box justifyContent="center" width={width}>
          <Text color={TEXT_DIM}>▼ more</Text>
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
  const isError =
    status.startsWith("error") || status.startsWith("failed");
  const statusColor = status === "ready" ? TEAL : isError ? CRANBERRY : TEXT_DIM;
  const inputWidth = Math.min(56, width - 8);

  return (
    <Box
      flexDirection="column"
      alignItems="center"
      justifyContent="center"
      width={width}
      height={height}
    >
      {/* Goose art */}
      <Box flexDirection="column" alignItems="center">
        {frame.map((line, i) => (
          <Text key={i} color={TEXT_PRIMARY}>
            {line}
          </Text>
        ))}
      </Box>

      {/* Title + subtitle */}
      <Box marginTop={1}>
        <Text color={TEXT_PRIMARY} bold>
          goose
        </Text>
      </Box>
      <Text color={TEXT_DIM}>your on-machine AI agent</Text>

      {/* Input or status */}
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

export default function App({
  serverUrl,
  initialPrompt,
}: {
  serverUrl: string;
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

  const turnsLen = turns.length;
  useEffect(() => {
    if (viewTurnIdx === -1) setScrollOffset(0);
  }, [turnsLen, viewTurnIdx]);

  const appendAgent = useCallback((text: string) => {
    setTurns((prev) => {
      if (prev.length === 0) return prev;
      const last = { ...prev[prev.length - 1]! };
      last.agentText = last.agentText + text;
      return [...prev.slice(0, -1), last];
    });
  }, []);

  const appendToolCall = useCallback((title: string) => {
    setTurns((prev) => {
      if (prev.length === 0) return prev;
      const last = { ...prev[prev.length - 1]! };
      last.toolCalls = [...last.toolCalls, title];
      return [...prev.slice(0, -1), last];
    });
  }, []);

  const addUserTurn = useCallback((text: string) => {
    setTurns((prev) => [
      ...prev,
      { userText: text, toolCalls: [], agentText: "" },
    ]);
    setViewTurnIdx(-1);
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

  const processQueue = useCallback(async () => {
    if (isProcessingRef.current) return;
    isProcessingRef.current = true;

    while (queueRef.current.length > 0) {
      const next = queueRef.current.shift()!;
      setQueuedMessages([...queueRef.current]);

      const client = clientRef.current;
      const sid = sessionIdRef.current;
      if (!client || !sid) break;

      addUserTurn(next);
      setLoading(true);
      setStatus("thinking…");
      streamBuf.current = "";

      try {
        const result = await client.prompt({
          sessionId: sid,
          prompt: [{ type: "text", text: next }],
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
    }

    isProcessingRef.current = false;
  }, [appendAgent, addUserTurn]);

  const sendPrompt = useCallback(
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
        if (queueRef.current.length > 0) processQueue();
      }
    },
    [appendAgent, addUserTurn, processQueue],
  );

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        setStatus("initializing…");
        const stream = createHttpStream(serverUrl);

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
                appendToolCall(update.title || "tool");
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
          stream,
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
          if (initialPrompt) setTimeout(() => exit(), 100);
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
  }, [serverUrl, initialPrompt, sendPrompt, appendAgent, appendToolCall, exit]);

  const handleSubmit = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (!trimmed) return;
      setInput("");
      setViewTurnIdx(-1);
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

    // Permission navigation
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

    // Turn navigation: shift+arrow
    if (key.upArrow && key.shift) {
      setTurns((currentTurns) => {
        if (currentTurns.length <= 1) return currentTurns;
        setViewTurnIdx((prev) => {
          const effectiveIdx = prev === -1 ? currentTurns.length - 1 : prev;
          setScrollOffset(0);
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
          setScrollOffset(0);
          return next >= currentTurns.length ? -1 : next;
        });
        return currentTurns;
      });
      return;
    }

    // Scroll within turn
    if (key.pageUp || (key.upArrow && key.meta)) {
      setScrollOffset((prev) => Math.max(prev - 5, 0));
      return;
    }
    if (key.pageDown || (key.downArrow && key.meta)) {
      setScrollOffset((prev) => prev + 5);
      return;
    }
  });

  // ── Layout math ───────────────────────────────────────────────────────
  //
  // The vertical budget is:
  //   header (2 lines: title row + rule)
  //   user prompt (2 lines: blank line above + prompt text)
  //   body (flex: remaining space)
  //   input bar (dynamic: border top/bottom + wrapped content lines + margin bottom) — absent in pipe mode

  const GUTTER = 2;
  const innerWidth = Math.max(termWidth - GUTTER * 2, 20);
  const headerLines = 2;
  const userPromptLines = 2;
  const inputLines = initialPrompt
    ? 0
    : inputBarHeight(input, innerWidth, queuedMessages.length > 0);
  const bodyHeight = Math.max(
    termHeight - headerLines - userPromptLines - inputLines - 1,
    3,
  );

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

          <Box flexDirection="column" flexGrow={1} height={bodyHeight}>
            <TurnResponseBody
              turn={currentTurn}
              width={innerWidth}
              height={
                bodyHeight -
                (isLatest && queuedMessages.length > 0
                  ? queuedMessages.length
                  : 0)
              }
              scrollOffset={scrollOffset}
              loading={isLatest && loading}
              status={status}
              spinIdx={spinIdx}
              pendingPermission={isLatest ? pendingPermission : null}
              permissionIdx={permissionIdx}
            />

            {isLatest &&
              queuedMessages.map((text, i) => (
                <QueuedMessage key={`q-${i}`} text={text} />
              ))}
          </Box>
        </>
      ) : (
        <Box
          flexDirection="column"
          flexGrow={1}
          height={bodyHeight + userPromptLines}
        />
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
