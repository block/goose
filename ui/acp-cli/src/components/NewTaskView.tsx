import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';

interface NewTaskViewProps {
  onSubmit: (name: string, task: string) => void;
  onCancel: () => void;
}

export const NewTaskView: React.FC<NewTaskViewProps> = ({ onSubmit, onCancel }) => {
  const [step, setStep] = useState<'name' | 'task'>('name');
  const [name, setName] = useState('');
  const [task, setTask] = useState('');

  const handleNameSubmit = (value: string) => {
    if (value.trim()) {
      setName(value.trim());
      setStep('task');
    }
  };

  const handleTaskSubmit = (value: string) => {
    if (value.trim()) {
      onSubmit(name, value.trim());
    }
  };

  return (
    <Box flexDirection="column" padding={1}>
      <Box borderStyle="single" borderColor="green" paddingX={1} marginBottom={1}>
        <Text bold color="green">🆕 New Workstream</Text>
        <Text dimColor> | Press Esc to cancel</Text>
      </Box>

      <Box flexDirection="column" paddingX={1}>
        {step === 'name' ? (
          <>
            <Text>Enter a short name for this workstream (e.g., "fix-login-bug"):</Text>
            <Box marginTop={1}>
              <Text color="green" bold>{'> '}</Text>
              <TextInput
                value={name}
                onChange={setName}
                onSubmit={handleNameSubmit}
                placeholder="workstream-name"
              />
            </Box>
          </>
        ) : (
          <>
            <Box marginBottom={1}>
              <Text dimColor>Name: </Text>
              <Text color="cyan">{name}</Text>
            </Box>
            <Text>Describe the task for this workstream:</Text>
            <Box marginTop={1}>
              <Text color="green" bold>{'> '}</Text>
              <TextInput
                value={task}
                onChange={setTask}
                onSubmit={handleTaskSubmit}
                placeholder="What should this goose work on?"
              />
            </Box>
          </>
        )}
      </Box>

      <Box marginTop={2} paddingX={1}>
        <Text dimColor>
          {step === 'name' 
            ? 'A git worktree will be created for isolated work (if in a git repo)'
            : 'The goose will start working on this task immediately'}
        </Text>
      </Box>
    </Box>
  );
};
