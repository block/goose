import React from "react";
import { Box, Text } from "ink";
import { Spinner } from "./Spinner.js";
import { Rule } from "./Rule.js";
import { TEAL, CRANBERRY, TEXT_PRIMARY, TEXT_DIM, RULE_COLOR } from "../colors.js";
import { isErrorStatus } from "../utils.js";

interface HeaderProps {
  width: number;
  status: string;
  loading: boolean;
  spinIdx: number;
  providerId?: string | null;
  modelId?: string | null;
  turnInfo?: { current: number; total: number };
}

export const Header = React.memo(function Header({
  width,
  status,
  loading,
  spinIdx,
  providerId,
  modelId,
  turnInfo,
}: HeaderProps) {
  const statusColor =
    status === "ready" ? TEAL : isErrorStatus(status) ? CRANBERRY : TEXT_DIM;

  const constrainedWidth = Math.max(width, 20);
  const leftSideWidth = Math.min(Math.floor(constrainedWidth * 0.7), constrainedWidth - 15);
  const rightSideWidth = constrainedWidth - leftSideWidth;

  // Active provider/model next to the status. Show "provider · model" when it
  // fits; otherwise show the model, pre-truncated with an ellipsis on narrow
  // terminals (per AGENTS.md Ink-Text) rather than hiding it entirely.
  const provider = providerId || undefined;
  const model = modelId || undefined;
  const modelOnly = model ?? provider;
  const full = provider && model ? `${provider} · ${model}` : modelOnly;
  // Space left for the label after the "goose · " prefix, the status, its
  // " · " separator, and the spinner — so the pre-truncation matches the render.
  const spinnerWidth = loading ? 2 : 0;
  const reserved =
    "goose · ".length + status.length + " · ".length + spinnerWidth;
  const avail = Math.max(0, leftSideWidth - reserved);
  let modelLabel: string | undefined;
  if (full && full.length <= avail) {
    modelLabel = full;
  } else if (modelOnly && avail >= 2) {
    // Cut on code points, not UTF-16 units: slicing an astral character in half
    // would leave a lone surrogate that renders as a replacement glyph.
    modelLabel =
      modelOnly.length <= avail
        ? modelOnly
        : [...modelOnly].slice(0, avail - 1).join("") + "…";
  }

  return (
    <Box flexDirection="column" width={constrainedWidth} flexShrink={0}>
      <Box justifyContent="space-between" width={constrainedWidth}>
        <Box width={leftSideWidth}>
          <Text color={TEXT_PRIMARY} bold>goose</Text>
          {modelLabel && (
            <>
              <Text color={RULE_COLOR}> · </Text>
              <Text color={TEXT_DIM} wrap="truncate-end">{modelLabel}</Text>
            </>
          )}
          <Text color={RULE_COLOR}> · </Text>
          <Box flexShrink={1}>
            <Text color={statusColor} wrap="truncate-end">{status}</Text>
          </Box>
          {loading && (
            <Text> <Spinner idx={spinIdx} /></Text>
          )}
        </Box>
        <Box width={rightSideWidth} justifyContent="flex-end">
          {turnInfo && turnInfo.total > 1 && (
            <Text color={TEXT_DIM}>
              {turnInfo.current}/{turnInfo.total}{"  "}
            </Text>
          )}
          <Text color={TEXT_DIM}>^E exts · ^M models · ^P providers</Text>
        </Box>
      </Box>
      <Rule width={constrainedWidth} />
    </Box>
  );
});
