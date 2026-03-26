import React from "react";
import { Box, Text } from "ink";
import { TEXT_PRIMARY, TEXT_SECONDARY, TEXT_DIM, GOLD, RULE_COLOR } from "./colors.js";

export interface SelectOption {
  id: string;
  name: string;
  description?: string;
}

interface SelectModalProps {
  title: string;
  options: SelectOption[];
  selectedIdx: number;
  filter: string;
  width: number;
  onClose?: () => void;
}

export function SelectModal({
  title,
  options,
  selectedIdx,
  filter,
  width,
}: SelectModalProps) {
  const filtered = filterOptions(options, filter);
  const modalWidth = Math.min(width - 4, 60);

  return (
    <Box
      flexDirection="column"
      marginLeft={2}
      marginTop={1}
      paddingX={2}
      paddingY={1}
      borderStyle="round"
      borderColor={GOLD}
      width={modalWidth}
    >
      <Text color={GOLD} bold>
        {title}
      </Text>

      {filter && (
        <Box marginTop={1}>
          <Text color={TEXT_DIM}>Filter: </Text>
          <Text color={TEXT_PRIMARY}>{filter}</Text>
        </Box>
      )}

      <Box marginTop={1} flexDirection="column">
        {filtered.length === 0 ? (
          <Text color={TEXT_DIM} italic>
            No matches found
          </Text>
        ) : (
          filtered.map((opt, i) => {
            const active = i === selectedIdx;
            return (
              <Box key={opt.id} flexDirection="column">
                <Box>
                  <Text color={active ? GOLD : RULE_COLOR}>
                    {active ? " ▸ " : "   "}
                  </Text>
                  <Text
                    color={active ? TEXT_PRIMARY : TEXT_SECONDARY}
                    bold={active}
                  >
                    {opt.name}
                  </Text>
                </Box>
                {opt.description && active && (
                  <Box paddingLeft={4}>
                    <Text color={TEXT_DIM}>{opt.description}</Text>
                  </Box>
                )}
              </Box>
            );
          })
        )}
      </Box>

      <Box marginTop={1}>
        <Text color={TEXT_DIM}>
          ↑↓ select · enter confirm · esc cancel · type to filter
        </Text>
      </Box>
    </Box>
  );
}

export function filterOptions(
  options: SelectOption[],
  filter: string,
): SelectOption[] {
  if (!filter) return options;
  
  const searchTerm = filter.toLowerCase();
  return options.filter(
    (opt) =>
      opt.name.toLowerCase().includes(searchTerm) ||
      opt.id.toLowerCase().includes(searchTerm) ||
      (opt.description && opt.description.toLowerCase().includes(searchTerm)),
  );
}
