import React from 'react';
import { Box, Text } from 'ink';

interface DiffViewProps {
  workstreamName: string;
  diff: string;
  onClose: () => void;
}

export const DiffView: React.FC<DiffViewProps> = ({ workstreamName, diff, onClose }) => {
  const lines = diff.split('\n').slice(0, 50); // Limit lines shown

  return (
    <Box flexDirection="column" padding={1}>
      <Box borderStyle="single" borderColor="yellow" paddingX={1} marginBottom={1}>
        <Text bold color="yellow">📝 Git Diff - {workstreamName}</Text>
        <Text dimColor> | Press any key to close</Text>
      </Box>

      <Box flexDirection="column" paddingX={1} flexGrow={1}>
        {diff.trim() === '' ? (
          <Text dimColor>No changes detected</Text>
        ) : (
          lines.map((line, i) => {
            let color: string | undefined;
            if (line.startsWith('+') && !line.startsWith('+++')) {
              color = 'green';
            } else if (line.startsWith('-') && !line.startsWith('---')) {
              color = 'red';
            } else if (line.startsWith('@@')) {
              color = 'cyan';
            } else if (line.startsWith('diff ') || line.startsWith('index ')) {
              color = 'yellow';
            }
            return (
              <Text key={i} color={color}>
                {line}
              </Text>
            );
          })
        )}
        {lines.length >= 50 && (
          <Text dimColor>... (truncated, {diff.split('\n').length - 50} more lines)</Text>
        )}
      </Box>
    </Box>
  );
};

interface StatusViewProps {
  workstreamName: string;
  status: string;
  onClose: () => void;
}

export const StatusView: React.FC<StatusViewProps> = ({ workstreamName, status, onClose }) => {
  const lines = status.split('\n').filter(Boolean);

  return (
    <Box flexDirection="column" padding={1}>
      <Box borderStyle="single" borderColor="blue" paddingX={1} marginBottom={1}>
        <Text bold color="blue">📋 Git Status - {workstreamName}</Text>
        <Text dimColor> | Press any key to close</Text>
      </Box>

      <Box flexDirection="column" paddingX={1}>
        {status.trim() === '' ? (
          <Text dimColor>Working tree clean</Text>
        ) : (
          lines.map((line, i) => {
            const status = line.slice(0, 2);
            const file = line.slice(3);
            let color: string | undefined;
            
            if (status.includes('M')) color = 'yellow';
            else if (status.includes('A')) color = 'green';
            else if (status.includes('D')) color = 'red';
            else if (status.includes('?')) color = 'gray';
            
            return (
              <Text key={i}>
                <Text color={color}>{status}</Text>
                <Text> {file}</Text>
              </Text>
            );
          })
        )}
      </Box>
    </Box>
  );
};
