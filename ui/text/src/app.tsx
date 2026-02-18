import React, { useState, useEffect, useCallback, useRef } from "react";
import { Box, Text, useApp, useInput, useStdout } from "ink";
import TextInput from "ink-text-input";
import {
  ClientSideConnection,
  type SessionNotification,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
} from "@agentclientprotocol/sdk";
import { createHttpStream } from "./transport.js";

// ── New England palette ─────────────────────────────────────────────
const NAVY = "#1B3A5C";
const CRANBERRY = "#9B2335";
const SLATE = "#708090";
const FOREST = "#2D5A3D";
const IVORY = "#D4C9A8";
const WARM_WHITE = "#E8E0D0";

// ── ASCII goose waddle frames ────────────────────────────────────────
// Side-view goose waddling: body rocks left/right, feet alternate
const GOOSE_FRAMES = [
  // Frame 0: center, feet together
  [
    "    ,_",
    "   (o >",
    "   //\\",
    "   \\\\ \\",
    "    \\\\_/",
    "     |  |",
    "     ^ ^",
  ],
  // Frame 1: lean right, left foot forward
  [
    "     ,_",
    "    (o >",
    "    //\\",
    "    \\\\ \\",
    "     \\\\_/",
    "    /  |",
    "   ^   ^",
  ],
  // Frame 2: center, feet together
  [
    "    ,_",
    "   (o >",
    "   //\\",
    "   \\\\ \\",
    "    \\\\_/",
    "     |  |",
    "     ^  ^",
  ],
  // Frame 3: lean left, right foot forward
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

const TITLE_ART = [
  "   __ _ ___   ___  ___  ___",
  "  / _` / _ \\ / _ \\/ __|/ _ \\",
  " | (_| | (_) | (_) \\__ \\  __/",
  "  \\__, |\\___/ \\___/|___/\\___|",
  "  |___/",
];

// ── Spinner ─────────────────────────────────────────────────────────
const SPINNER_FRAMES = ["◐", "◓", "◑", "◒"];

// ── Message types ───────────────────────────────────────────────────
interface TextMessage {
  kind: "text";
  role: "user" | "agent";
  text: string;
}

interface ToolCallMessage {
  kind: "tool_call";
  title: string;
}

type Message = TextMessage | ToolCallMessage;

// ── Thin separator ──────────────────────────────────────────────────
function Separator({ width }: { width: number }) {
  const line = "─".repeat(Math.max(width - 4, 20));
  return (
    <Box marginY={0} paddingLeft={1}>
      <Text color={SLATE} dimColor>
        {line}
      </Text>
    </Box>
  );
}

// ── Tool call renderer ──────────────────────────────────────────────
function ToolCallBlock({ title }: { title: string }) {
  return (
    <Box
      marginLeft={2}
      marginY={0}
      paddingX={1}
      borderStyle="round"
      borderColor={SLATE}
      borderDimColor
    >
      <Text color={FOREST}>⚙ </Text>
      <Text color={SLATE} italic>
        {title}
      </Text>
    </Box>
  );
}

// ── User message renderer ───────────────────────────────────────────
function UserMessage({ text }: { text: string }) {
  return (
    <Box marginBottom={0} paddingLeft={1}>
      <Text color={CRANBERRY} bold>
        ❯{" "}
      </Text>
      <Text color={WARM_WHITE}>{text}</Text>
    </Box>
  );
}

// ── Agent message renderer ──────────────────────────────────────────
function AgentMessage({ text }: { text: string }) {
  return (
    <Box marginBottom={0} paddingLeft={3}>
      <Text color={IVORY}>{text}</Text>
    </Box>
  );
}

// ── Goose banner with animation ─────────────────────────────────────
function GooseBanner({
  animFrame,
  showTitle,
}: {
  animFrame: number;
  showTitle: boolean;
}) {
  const frame = GOOSE_FRAMES[animFrame % GOOSE_FRAMES.length]!;
  return (
    <Box flexDirection="row" gap={2} marginBottom={0}>
      <Box flexDirection="column">
        {frame.map((line, i) => (
          <Text key={i} color={NAVY}>
            {line}
          </Text>
        ))}
      </Box>
      {showTitle && (
        <Box flexDirection="column" justifyContent="center">
          {TITLE_ART.map((line, i) => (
            <Text key={i} color={CRANBERRY} bold>
              {line}
            </Text>
          ))}
          <Text color={SLATE} dimColor>
            {"  "}an open-source AI agent
          </Text>
        </Box>
      )}
    </Box>
  );
}

// ── Main App ────────────────────────────────────────────────────────
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

  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState("connecting...");
  const [spinIdx, setSpinIdx] = useState(0);
  const [gooseFrame, setGooseFrame] = useState(0);
  const [bannerVisible, setBannerVisible] = useState(true);
  const connRef = useRef<ClientSideConnection | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const streamBuf = useRef("");
  const sentInitialPrompt = useRef(false);

  // Spinner + goose animation
  useEffect(() => {
    const t = setInterval(() => {
      setSpinIdx((i) => (i + 1) % SPINNER_FRAMES.length);
      setGooseFrame((f) => f + 1);
    }, 300);
    return () => clearInterval(t);
  }, []);

  // Hide banner once first message arrives
  useEffect(() => {
    if (messages.length > 0) {
      setBannerVisible(false);
    }
  }, [messages]);

  const appendAgent = useCallback((text: string) => {
    setMessages((prev) => {
      const last = prev[prev.length - 1];
      if (last && last.kind === "text" && last.role === "agent") {
        return [
          ...prev.slice(0, -1),
          { kind: "text" as const, role: "agent" as const, text: last.text + text },
        ];
      }
      return [...prev, { kind: "text" as const, role: "agent" as const, text }];
    });
  }, []);

  const appendToolCall = useCallback((title: string) => {
    setMessages((prev) => [...prev, { kind: "tool_call" as const, title }]);
  }, []);

  const sendPrompt = useCallback(
    async (text: string) => {
      const conn = connRef.current;
      const sid = sessionIdRef.current;
      if (!conn || !sid) return;

      setMessages((prev) => [
        ...prev,
        { kind: "text" as const, role: "user" as const, text },
      ]);
      setLoading(true);
      setStatus("thinking...");
      streamBuf.current = "";

      try {
        const result = await conn.prompt({
          sessionId: sid,
          prompt: [{ type: "text", text }],
        });

        if (streamBuf.current) {
          appendAgent("");
        }

        setStatus(
          result.stopReason === "end_turn" ? "ready" : `stopped: ${result.stopReason}`
        );
      } catch (e: unknown) {
        const errMsg = e instanceof Error ? e.message : String(e);
        setStatus(`error: ${errMsg}`);
      } finally {
        setLoading(false);
      }
    },
    [appendAgent]
  );

  // Initialize connection
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        setStatus("initializing...");
        const stream = createHttpStream(serverUrl);

        const conn = new ClientSideConnection(
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
              _params: RequestPermissionRequest
            ): Promise<RequestPermissionResponse> => ({
              outcome: { outcome: "cancelled" },
            }),
          }),
          stream
        );

        if (cancelled) return;
        connRef.current = conn;

        setStatus("handshaking...");
        await conn.initialize({
          protocolVersion: "0",
          clientInfo: { name: "goose-text", version: "0.1.0" },
          clientCapabilities: {},
        });

        if (cancelled) return;

        setStatus("creating session...");
        const session = await conn.newSession({
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
          if (initialPrompt) {
            setTimeout(() => exit(), 100);
          }
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
      if (!trimmed || loading) return;
      setInput("");
      sendPrompt(trimmed);
    },
    [loading, sendPrompt]
  );

  useInput((ch, key) => {
    if (key.escape || (ch === "c" && key.ctrl)) {
      exit();
    }
  });

  // ── Status bar color ────────────────────────────────────────────
  const statusColor =
    status === "ready"
      ? FOREST
      : status.startsWith("error") || status.startsWith("failed")
        ? CRANBERRY
        : SLATE;

  return (
    <Box flexDirection="column" paddingX={1} paddingY={1}>
      {/* ── Header ─────────────────────────────────────────────── */}
      {bannerVisible ? (
        <GooseBanner animFrame={gooseFrame} showTitle />
      ) : (
        <Box marginBottom={0}>
          <Text color={NAVY} bold>
            goose
          </Text>
          <Text color={SLATE}> │ </Text>
          <Text color={statusColor}>{status}</Text>
          {loading && (
            <Text color={CRANBERRY}>
              {" "}
              {SPINNER_FRAMES[spinIdx % SPINNER_FRAMES.length]}
            </Text>
          )}
        </Box>
      )}

      <Separator width={termWidth} />

      {/* ── Messages ───────────────────────────────────────────── */}
      {messages.map((msg, i) => {
        if (msg.kind === "tool_call") {
          return <ToolCallBlock key={i} title={msg.title} />;
        }
        if (msg.role === "user") {
          return (
            <React.Fragment key={i}>
              {i > 0 && <Separator width={termWidth} />}
              <UserMessage text={msg.text} />
              <Separator width={termWidth} />
            </React.Fragment>
          );
        }
        return <AgentMessage key={i} text={msg.text} />;
      })}

      {/* ── Loading indicator ──────────────────────────────────── */}
      {loading && messages.length > 0 && (
        <Box paddingLeft={3} marginTop={0}>
          <Text color={CRANBERRY}>
            {SPINNER_FRAMES[spinIdx % SPINNER_FRAMES.length]}{" "}
          </Text>
          <Text color={SLATE} italic>
            {status}
          </Text>
        </Box>
      )}

      {/* ── Input ──────────────────────────────────────────────── */}
      {!loading && !initialPrompt && (
        <Box marginTop={messages.length > 0 ? 1 : 0} paddingLeft={1}>
          <Text color={CRANBERRY} bold>
            ❯{" "}
          </Text>
          <TextInput value={input} onChange={setInput} onSubmit={handleSubmit} />
        </Box>
      )}
    </Box>
  );
}
