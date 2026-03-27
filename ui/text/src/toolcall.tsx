import React from "react";
import { Box, Text } from "ink";
import type {
  ToolCallContent,
  ToolCallStatus,
  ToolKind,
} from "@agentclientprotocol/sdk";
import { CRANBERRY, TEAL, GOLD, TEXT_SECONDARY, TEXT_DIM } from "./colors.js";

export interface ToolCallInfo {
  toolCallId: string;
  title: string;
  status: ToolCallStatus;
  kind?: ToolKind;
  rawInput?: unknown;
  rawOutput?: unknown;
  content?: ToolCallContent[];
  locations?: Array<{ path: string; line?: number | null }>;
}

const CEDAR = "#6B5344";

const KIND_ICONS: Record<string, string> = {
  read: "📖",
  edit: "✏️",
  delete: "🗑",
  move: "📦",
  search: "🔍",
  execute: "▶",
  think: "💭",
  fetch: "🌐",
  switch_mode: "🔀",
  other: "⚙",
};

const STATUS_INDICATORS: Record<string, { icon: string; color: string }> = {
  pending: { icon: "○", color: TEXT_DIM },
  in_progress: { icon: "◑", color: GOLD },
  completed: { icon: "●", color: TEAL },
  failed: { icon: "✗", color: CRANBERRY },
};

function formatJsonCompact(value: unknown, maxWidth: number): string[] {
  if (value === undefined || value === null) return [];
  let raw: string;
  try {
    raw = JSON.stringify(value, null, 2);
  } catch {
    raw = String(value);
  }
  const lines = raw.split("\n");
  const result: string[] = [];
  for (const line of lines) {
    if (line.length <= maxWidth) {
      result.push(line);
    } else {
      let remaining = line;
      while (remaining.length > maxWidth) {
        result.push(remaining.slice(0, maxWidth));
        remaining = remaining.slice(maxWidth);
      }
      if (remaining) result.push(remaining);
    }
  }
  return result;
}

function extractTextFromContent(content: ToolCallContent[]): string[] {
  const lines: string[] = [];
  for (const item of content) {
    if (item.type === "content" && item.content) {
      const block = item.content as any;
      if (block.type === "text" && block.text) {
        lines.push(...block.text.split("\n"));
      }
    } else if (item.type === "diff") {
      const diff = item as any;
      lines.push(`diff: ${diff.path || "unknown"}`);
    } else if (item.type === "terminal") {
      const term = item as any;
      lines.push(`terminal: ${term.terminalId || "unknown"}`);
    }
  }
  return lines;
}

function summarizeContent(info: ToolCallInfo): string {
  const parts: string[] = [];

  if (info.locations && info.locations.length > 0) {
    for (const loc of info.locations) {
      parts.push(loc.path + (loc.line ? `:${loc.line}` : ""));
    }
  }

  if (info.content && info.content.length > 0) {
    const textLines = extractTextFromContent(info.content);
    if (textLines.length > 0) {
      const first = textLines[0]!.trim();
      if (first.length > 60) {
        parts.push(first.slice(0, 57) + "…");
      } else if (first) {
        parts.push(first);
      }
    }
  }

  if (parts.length === 0 && info.rawOutput !== undefined && info.rawOutput !== null) {
    const raw = String(
      typeof info.rawOutput === "string" ? info.rawOutput : JSON.stringify(info.rawOutput),
    );
    const firstLine = raw.split("\n")[0] ?? "";
    if (firstLine.length > 60) {
      parts.push(firstLine.slice(0, 57) + "…");
    } else if (firstLine) {
      parts.push(firstLine);
    }
  }

  return parts.join(" · ");
}

export function findFeaturedToolCallId(
  toolCallOrder: string[],
  toolCalls: Map<string, ToolCallInfo>,
): string | undefined {
  for (let i = toolCallOrder.length - 1; i >= 0; i--) {
    const tc = toolCalls.get(toolCallOrder[i]!);
    if (tc && (tc.status === "pending" || tc.status === "in_progress")) {
      return toolCallOrder[i]!;
    }
  }
  return toolCallOrder[toolCallOrder.length - 1];
}

interface ToolCallProps {
  info: ToolCallInfo;
  width: number;
  expanded: boolean;
  showTabHint: boolean;
  keyPrefix: string;
}

export function ToolCallCard({
  info,
  width,
  expanded,
  showTabHint,
  keyPrefix,
}: ToolCallProps): React.ReactNode[] {
  const kindIcon = KIND_ICONS[info.kind ?? "other"] ?? "⚙";
  const statusInfo = STATUS_INDICATORS[info.status] ?? STATUS_INDICATORS.pending!;
  const borderColor = info.status === "failed" ? CRANBERRY : CEDAR;
  const dimBorder = info.status !== "failed";

  const hasInput = info.rawInput !== undefined && info.rawInput !== null;
  const hasOutput = info.rawOutput !== undefined && info.rawOutput !== null;
  const hasContent = info.content && info.content.length > 0;
  const hasLocations = info.locations && info.locations.length > 0;

  const contentWidth = width - 4;

  const lines: React.ReactNode[] = [];
  const content: React.ReactNode[] = [];

  // Header
  const runningText = info.status === "in_progress" ? " running…" : "";
  content.push(
    <Box key="header" flexDirection="row">
      <Text color={statusInfo.color}>{statusInfo.icon}</Text>
      <Text> </Text>
      <Text>{kindIcon}</Text>
      <Text> </Text>
      <Text color={TEXT_SECONDARY} bold>{info.title}</Text>
      {runningText && <Text color={TEXT_DIM} italic>{runningText}</Text>}
      <Box flexGrow={1} />
      {showTabHint && !expanded && <Text color={TEXT_DIM} italic>tab ↔</Text>}
    </Box>
  );

  // Compact view - show summary
  if (!expanded) {
    const summary = summarizeContent(info);
    if (summary) {
      content.push(
        <Box key="summary">
          <Text color={TEXT_DIM}>{summary}</Text>
        </Box>
      );
    }
  } else {
    // Expanded view - show all details
    const inputLines = hasInput ? formatJsonCompact(info.rawInput, contentWidth - 6) : [];
    const outputLines = hasOutput ? formatJsonCompact(info.rawOutput, contentWidth - 6) : [];
    const contentLines = hasContent ? extractTextFromContent(info.content!) : [];

    if (hasLocations) {
      for (let i = 0; i < info.locations!.length; i++) {
        const loc = info.locations![i]!;
        content.push(
          <Box key={`loc-${i}`}>
            <Text color={TEXT_DIM}>📁 {loc.path}{loc.line ? `:${loc.line}` : ""}</Text>
          </Box>
        );
      }
    }

    const addSection = (label: string, sectionLines: string[]) => {
      if (sectionLines.length === 0) return;
      
      content.push(
        <Box key={`${label}-header`}>
          <Text color={TEXT_DIM}>▸ {label}:</Text>
        </Box>
      );

      for (let i = 0; i < sectionLines.length; i++) {
        content.push(
          <Box key={`${label}-${i}`} paddingLeft={2}>
            <Text color={TEXT_DIM}>{sectionLines[i]}</Text>
          </Box>
        );
      }
    };

    addSection("input", inputLines);
    addSection("output", outputLines);
    addSection("content", contentLines);
  }

  lines.push(
    <Box
      key={keyPrefix}
      width={width}
      flexDirection="column"
      borderStyle="round"
      borderColor={borderColor}
      borderDimColor={dimBorder}
      paddingX={1}
    >
      {content}
    </Box>
  );

  return lines;
}
