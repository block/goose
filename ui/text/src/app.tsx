import React, { useState, useEffect, useCallback, useRef } from "react";
import { Box, Text, useApp, useInput, useStdout } from "ink";
import TextInput from "ink-text-input";
import {
  ClientSideConnection,
  type SessionNotification,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
} from "@agentclientprotocol/sdk";
import { GooseClient } from "@goose/acp";
import { createHttpStream } from "./transport.js";

interface PendingPermission {
  toolTitle: string;
  options: Array<{ optionId: string; name: string; kind: string }>;
  resolve: (response: RequestPermissionResponse) => void;
}

const CRANBERRY_BRIGHT = "#C0354A";
const HARBOR_NAVY = "#1B2A4A";
const DEEP_SLATE = "#3A4F6F";
const SLATE = "#6B7F99";
const LIGHT_SLATE = "#8FA4BD";
const AUTUMN_GOLD = "#C4883A";
const OCEAN_TEAL = "#3A7D7B";
const CEDAR_BROWN = "#6B5344";
const FOG_WHITE = "#E8E4DF";
const PARCHMENT = "#D4CFC8";

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

const TITLE_TEXT = "goose";

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

function HRule({ width, color }: { width: number; color?: string }) {
  return (
    <Box>
      <Text color={color ?? DEEP_SLATE} dimColor>
        {"─".repeat(Math.max(width, 1))}
      </Text>
    </Box>
  );
}

function HeaderBar({
  width,
  status,
  loading,
  spinIdx,
  hasPendingPermission,
}: {
  width: number;
  status: string;
  loading: boolean;
  spinIdx: number;
  hasPendingPermission: boolean;
}) {
  const statusColor =
    status === "ready"
      ? OCEAN_TEAL
      : status.startsWith("error") || status.startsWith("failed")
        ? CRANBERRY_BRIGHT
        : SLATE;

  const leftContent = ` ${TITLE_TEXT} `;
  const spinner =
    loading && !hasPendingPermission
      ? ` ${SPINNER_FRAMES[spinIdx % SPINNER_FRAMES.length]} `
      : "";

  return (
    <Box flexDirection="column" width={width}>
      <Box justifyContent="space-between" width={width}>
        <Box>
          <Text color={FOG_WHITE} bold>
            {leftContent}
          </Text>
          <Text color={DEEP_SLATE}>│</Text>
          <Text color={statusColor}> {status}</Text>
          {spinner && <Text color={CRANBERRY_BRIGHT}>{spinner}</Text>}
        </Box>
        <Box>
          <Text color={DEEP_SLATE} dimColor>
            ctrl+c to exit{" "}
          </Text>
        </Box>
      </Box>
      <HRule width={width} color={DEEP_SLATE} />
    </Box>
  );
}

function ToolCallBlock({ title, width }: { title: string; width: number }) {
  return (
    <Box
      marginLeft={3}
      marginY={0}
      paddingX={1}
      borderStyle="round"
      borderColor={CEDAR_BROWN}
      borderDimColor
      width={Math.min(width - 6, 72)}
    >
      <Text color={OCEAN_TEAL}>⚙ </Text>
      <Text color={LIGHT_SLATE} italic>
        {title}
      </Text>
    </Box>
  );
}

function UserMessage({ text, width }: { text: string; width: number }) {
  return (
    <Box flexDirection="column" width={width}>
      <Box paddingLeft={1} paddingY={0}>
        <Text color={CRANBERRY_BRIGHT} bold>
          {"❯ "}
        </Text>
        <Text color={FOG_WHITE} bold>
          {text}
        </Text>
      </Box>
    </Box>
  );
}

function AgentMessage({ text, width }: { text: string; width: number }) {
  return (
    <Box paddingLeft={3} paddingRight={2} width={width}>
      <Text color={PARCHMENT}>{text}</Text>
    </Box>
  );
}

function PermissionPrompt({
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
  return (
    <Box
      flexDirection="column"
      marginLeft={3}
      marginY={0}
      paddingX={2}
      paddingY={1}
      borderStyle="round"
      borderColor={AUTUMN_GOLD}
      width={Math.min(width - 6, 64)}
    >
      <Text color={AUTUMN_GOLD} bold>
        🔒 Permission required
      </Text>
      <Text color={FOG_WHITE}>{toolTitle}</Text>
      <Box marginTop={1} flexDirection="column">
        {options.map((opt, i) => {
          const key = PERMISSION_KEYS[opt.kind] ?? String(i + 1);
          const label = PERMISSION_LABELS[opt.kind] ?? opt.name;
          const selected = i === selectedIdx;
          return (
            <Box key={opt.optionId}>
              <Text color={selected ? AUTUMN_GOLD : DEEP_SLATE}>
                {selected ? " ▸ " : "   "}
              </Text>
              <Text color={selected ? FOG_WHITE : LIGHT_SLATE} bold={selected}>
                [{key}] {label}
              </Text>
            </Box>
          );
        })}
      </Box>
      <Box marginTop={1}>
        <Text color={SLATE} dimColor>
          ↑↓ select · enter confirm · esc cancel
        </Text>
      </Box>
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
  const statusColor =
    status === "ready"
      ? OCEAN_TEAL
      : status.startsWith("error") || status.startsWith("failed")
        ? CRANBERRY_BRIGHT
        : SLATE;

  const inputWidth = Math.min(60, width - 8);

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
          <Text key={i} color={FOG_WHITE}>
            {line}
          </Text>
        ))}
      </Box>
      <Box marginTop={1}>
        <Text color={FOG_WHITE} bold>
          {TITLE_TEXT}
        </Text>
      </Box>
      <Box marginTop={0}>
        <Text color={SLATE}>your on-machine AI agent</Text>
      </Box>

      {showInput ? (
        <>
          <Box marginTop={2}>
            <HRule width={inputWidth} color={DEEP_SLATE} />
          </Box>
          <Box marginTop={0}>
            <Text color={CRANBERRY_BRIGHT} bold>
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
          <Box marginTop={0}>
            <HRule width={inputWidth} color={DEEP_SLATE} />
          </Box>
        </>
      ) : (
        <>
          <Box marginTop={2}>
            <HRule width={Math.min(40, width - 4)} color={DEEP_SLATE} />
          </Box>
          <Box marginTop={1}>
            {loading && (
              <Text color={CRANBERRY_BRIGHT}>
                {SPINNER_FRAMES[spinIdx % SPINNER_FRAMES.length]}{" "}
              </Text>
            )}
            <Text color={statusColor}>{status}</Text>
          </Box>
        </>
      )}
    </Box>
  );
}

function InputBar({
  width,
  input,
  onChange,
  onSubmit,
}: {
  width: number;
  input: string;
  onChange: (v: string) => void;
  onSubmit: (v: string) => void;
}) {
  return (
    <Box flexDirection="column" width={width}>
      <HRule width={width} color={DEEP_SLATE} />
      <Box paddingLeft={1} paddingY={0}>
        <Text color={CRANBERRY_BRIGHT} bold>
          {"❯ "}
        </Text>
        <TextInput value={input} onChange={onChange} onSubmit={onSubmit} />
      </Box>
    </Box>
  );
}

function LoadingIndicator({
  status,
  spinIdx,
}: {
  status: string;
  spinIdx: number;
}) {
  return (
    <Box paddingLeft={3} marginTop={0}>
      <Text color={CRANBERRY_BRIGHT}>
        {SPINNER_FRAMES[spinIdx % SPINNER_FRAMES.length]}{" "}
      </Text>
      <Text color={SLATE} italic>
        {status}
      </Text>
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

  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState("connecting...");
  const [spinIdx, setSpinIdx] = useState(0);
  const [gooseFrame, setGooseFrame] = useState(0);
  const [bannerVisible, setBannerVisible] = useState(true);
  const [pendingPermission, setPendingPermission] =
    useState<PendingPermission | null>(null);
  const [permissionIdx, setPermissionIdx] = useState(0);
  const connRef = useRef<ClientSideConnection | null>(null);
  const gooseRef = useRef<GooseClient | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const streamBuf = useRef("");
  const sentInitialPrompt = useRef(false);

  useEffect(() => {
    const t = setInterval(() => {
      setSpinIdx((i) => (i + 1) % SPINNER_FRAMES.length);
      setGooseFrame((f) => f + 1);
    }, 300);
    return () => clearInterval(t);
  }, []);

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
          {
            kind: "text" as const,
            role: "agent" as const,
            text: last.text + text,
          },
        ];
      }
      return [...prev, { kind: "text" as const, role: "agent" as const, text }];
    });
  }, []);

  const appendToolCall = useCallback((title: string) => {
    setMessages((prev) => [...prev, { kind: "tool_call" as const, title }]);
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
    [appendAgent],
  );

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
        connRef.current = conn;
        gooseRef.current = new GooseClient(conn);

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
        if (selected) {
          resolvePermission({ optionId: selected.optionId });
        }
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
        if (match) {
          resolvePermission({ optionId: match.optionId });
          return;
        }
      }
    }
  });

  const PAD_X = 2;
  const PAD_BOTTOM = 1;
  const innerWidth = Math.max(termWidth - PAD_X * 2, 20);
  // Header = 2 lines (title row + hrule), Input = 2 lines (hrule + input row)
  const headerHeight = 2;
  const inputBarHeight = initialPrompt ? 0 : 2;
  const bodyHeight = Math.max(termHeight - headerHeight - inputBarHeight - PAD_BOTTOM, 3);

  if (bannerVisible) {
    return (
      <Box
        flexDirection="column"
        width={termWidth}
        height={termHeight}
      >
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

  return (
    <Box
      flexDirection="column"
      width={termWidth}
      height={termHeight}
      paddingX={PAD_X}
      paddingBottom={PAD_BOTTOM}
    >
      <HeaderBar
        width={innerWidth}
        status={status}
        loading={loading}
        spinIdx={spinIdx}
        hasPendingPermission={!!pendingPermission}
      />

      <Box
        flexDirection="column"
        flexGrow={1}
        height={bodyHeight}
        overflowY="hidden"
        paddingY={0}
      >
        {messages.map((msg, i) => {
          if (msg.kind === "tool_call") {
            return <ToolCallBlock key={i} title={msg.title} width={innerWidth} />;
          }
          if (msg.role === "user") {
            return (
              <React.Fragment key={i}>
                {i > 0 && <Box height={1} />}
                <UserMessage text={msg.text} width={innerWidth} />
                <HRule width={innerWidth} color={HARBOR_NAVY} />
              </React.Fragment>
            );
          }
          return <AgentMessage key={i} text={msg.text} width={innerWidth} />;
        })}

        {pendingPermission && (
          <PermissionPrompt
            toolTitle={pendingPermission.toolTitle}
            options={pendingPermission.options}
            selectedIdx={permissionIdx}
            width={innerWidth}
          />
        )}

        {loading && !pendingPermission && messages.length > 0 && (
          <LoadingIndicator status={status} spinIdx={spinIdx} />
        )}
      </Box>

      {!loading && !pendingPermission && !initialPrompt && (
        <InputBar
          width={innerWidth}
          input={input}
          onChange={setInput}
          onSubmit={handleSubmit}
        />
      )}
    </Box>
  );
}
