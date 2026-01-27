import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';

interface PermissionOption {
  id: string;
  label: string;
  kind: string;
}

interface PermissionDialogProps {
  workstreamName: string;
  toolTitle: string;
  toolInput?: unknown;
  options: PermissionOption[];
  onSelect: (optionId: string) => void;
  onCancel: () => void;
}

export const PermissionDialog: React.FC<PermissionDialogProps> = ({
  workstreamName,
  toolTitle,
  toolInput,
  options,
  onSelect,
  onCancel
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  useInput((input, key) => {
    if (key.upArrow || input === 'k') {
      setSelectedIndex(Math.max(0, selectedIndex - 1));
    } else if (key.downArrow || input === 'j') {
      setSelectedIndex(Math.min(options.length - 1, selectedIndex + 1));
    } else if (key.return) {
      onSelect(options[selectedIndex].id);
    } else if (key.escape) {
      onCancel();
    } else if (input >= '1' && input <= '4') {
      const idx = parseInt(input) - 1;
      if (idx < options.length) {
        onSelect(options[idx].id);
      }
    }
  });

  const formatInput = (input: unknown): string => {
    if (!input) return '';
    try {
      const str = typeof input === 'string' ? input : JSON.stringify(input, null, 2);
      // Truncate long inputs
      if (str.length > 200) {
        return str.slice(0, 200) + '...';
      }
      return str;
    } catch {
      return String(input);
    }
  };

  return (
    <Box flexDirection="column" borderStyle="double" borderColor="yellow" padding={1}>
      <Box marginBottom={1}>
        <Text bold color="yellow">⚠️ Permission Required</Text>
      </Box>

      <Box marginBottom={1}>
        <Text>
          <Text dimColor>Workstream: </Text>
          <Text color="cyan">{workstreamName}</Text>
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text>
          <Text dimColor>Tool: </Text>
          <Text bold>{toolTitle}</Text>
        </Text>
      </Box>

      {toolInput !== undefined && toolInput !== null && (
        <Box marginBottom={1} flexDirection="column">
          <Text dimColor>Input:</Text>
          <Box marginLeft={2} borderStyle="single" borderColor="gray" paddingX={1}>
            <Text>{formatInput(toolInput)}</Text>
          </Box>
        </Box>
      )}

      <Box flexDirection="column" marginBottom={1}>
        <Text dimColor>Select an option:</Text>
        {options.map((option, i) => {
          const optionColor = getOptionColor(option.kind);
          return (
            <Box key={option.id} marginLeft={2}>
              <Text>
                {i === selectedIndex ? (
                  <Text color="cyan" bold>❯ </Text>
                ) : (
                  <Text>  </Text>
                )}
                <Text color={optionColor} bold={i === selectedIndex}>
                  [{i + 1}] {formatOptionLabel(option.kind)}
                </Text>
              </Text>
            </Box>
          );
        })}
      </Box>

      <Box>
        <Text dimColor>↑↓ to select, Enter to confirm, Esc to cancel</Text>
      </Box>
    </Box>
  );
};

function getOptionColor(kind: string): 'green' | 'red' | 'white' {
  switch (kind) {
    case 'allow_always':
    case 'allow_once':
      return 'green';
    case 'reject_always':
    case 'reject_once':
      return 'red';
    default:
      return 'white';
  }
}

function formatOptionLabel(kind: string): string {
  switch (kind) {
    case 'allow_always':
      return 'Allow Always';
    case 'allow_once':
      return 'Allow Once';
    case 'reject_always':
      return 'Reject Always';
    case 'reject_once':
      return 'Reject Once';
    default:
      return kind;
  }
}
