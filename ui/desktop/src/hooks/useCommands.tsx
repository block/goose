import { useCallback, useMemo } from 'react';
import { Zap } from 'lucide-react';

export interface Command {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  prompt: string;
  command: () => void;
}

export const BUILT_IN_COMMANDS: Command[] = [
  {
    id: 'compact',
    name: 'Compact',
    description: 'Compact the conversation to reduce context size',
    icon: <Zap size={16} />,
    prompt: 'Please compact this conversation',
    command: () => { /* TODO */ }
  },
];

export const useCommands = () => {
  // Return all commands (built-in for now, could include custom commands in the future)
  const commands = useMemo(() => BUILT_IN_COMMANDS, []);

  const getCommand = useCallback((id: string): Command | undefined => {
    return commands.find(cmd => cmd.id === id);
  }, [commands]);

  const getCommandByName = useCallback((name: string): Command | undefined => {
    return commands.find(cmd => cmd.name.toLowerCase() === name.toLowerCase());
  }, [commands]);

  const expandCommandPrompt = useCallback((command: Command, _context?: Record<string, unknown>): string => {
    return command.prompt;
  }, []);

  const incrementUsage = useCallback((_id: string) => {
    // TODO: Implement usage tracking if needed in the future
    // For now, this is a no-op
  }, []);

  return {
    commands,
    getCommand,
    getCommandByName,
    expandCommandPrompt,
    incrementUsage,
  };
};

export default useCommands;
