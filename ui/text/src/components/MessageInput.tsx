import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';

interface MessageInputProps {
  workstreamName: string;
  onSubmit: (message: string) => void;
  onCancel: () => void;
}

export const MessageInput: React.FC<MessageInputProps> = ({ 
  workstreamName, 
  onSubmit, 
  onCancel 
}) => {
  const [message, setMessage] = useState('');

  const handleSubmit = (value: string) => {
    if (value.trim()) {
      onSubmit(value.trim());
    }
  };

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="green" padding={1}>
      <Text dimColor>Send message to {workstreamName}:</Text>
      <Box marginTop={1}>
        <Text color="green" bold>{'> '}</Text>
        <TextInput
          value={message}
          onChange={setMessage}
          onSubmit={handleSubmit}
          placeholder="Type your message..."
        />
      </Box>
      <Box marginTop={1}>
        <Text dimColor>Press Enter to send, Esc to cancel</Text>
      </Box>
    </Box>
  );
};
