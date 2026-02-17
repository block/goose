import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { Loader2, Command, Slash } from 'lucide-react';
import { usePromptBar, SlashCommand } from '../../contexts/PromptBarContext';
import Send from '../icons/Send';

export default function PromptBar() {
  const location = useLocation();
  const promptBar = usePromptBar();
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showCommands, setShowCommands] = useState(false);
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const isOnPairRoute = location.pathname === '/pair';
  const isHidden = isOnPairRoute || !promptBar?.showPromptBar;

  const config = promptBar?.config;
  const slashCommands = useMemo(() => promptBar?.slashCommands ?? [], [promptBar?.slashCommands]);
  const submitPrompt = promptBar?.submitPrompt;

  // Filter commands based on input
  const filteredCommands: SlashCommand[] = input.startsWith('/')
    ? slashCommands.filter((cmd) =>
        cmd.command.toLowerCase().startsWith(input.toLowerCase())
      )
    : [];

  const handleSubmit = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || isLoading || !submitPrompt) return;

    // Check if it's a slash command
    if (trimmed.startsWith('/')) {
      const matchedCommand = slashCommands.find(
        (cmd) => trimmed.startsWith(cmd.command + ' ') || trimmed === cmd.command
      );
      if (matchedCommand) {
        matchedCommand.action(trimmed);
        setInput('');
        setShowCommands(false);
        return;
      }
    }

    // Default: submit as new session prompt
    setIsLoading(true);
    try {
      submitPrompt(trimmed);
      setInput('');
    } catch (error) {
      console.error('Failed to submit from prompt bar:', error);
    } finally {
      setIsLoading(false);
    }
  }, [input, isLoading, slashCommands, submitPrompt]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (showCommands && filteredCommands.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedCommandIndex((prev) =>
          prev < filteredCommands.length - 1 ? prev + 1 : 0
        );
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedCommandIndex((prev) =>
          prev > 0 ? prev - 1 : filteredCommands.length - 1
        );
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && showCommands)) {
        e.preventDefault();
        const selected = filteredCommands[selectedCommandIndex];
        if (selected) {
          setInput(selected.command + ' ');
          setShowCommands(false);
        }
        return;
      }
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
    if (e.key === 'Escape') {
      setShowCommands(false);
      inputRef.current?.blur();
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setInput(value);
    setShowCommands(value.startsWith('/'));
    setSelectedCommandIndex(0);
  };

  // Global Cmd/Ctrl+K shortcut to focus
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  if (isHidden) return null;

  return (
    <div className="fixed bottom-0 left-[var(--sidebar-width,0px)] right-0 z-50 pointer-events-none">
      {/* Command palette dropdown */}
      {showCommands && filteredCommands.length > 0 && (
        <div className="mx-4 mb-1 pointer-events-auto">
          <div className="bg-bgApp-secondary border border-borderSubtle rounded-lg shadow-lg overflow-hidden max-w-2xl mx-auto">
            {filteredCommands.map((cmd: SlashCommand, i: number) => (
              <button
                key={cmd.command}
                className={`w-full px-4 py-2.5 flex items-center gap-3 text-left transition-colors ${
                  i === selectedCommandIndex
                    ? 'bg-bgApp-active text-textProminent'
                    : 'text-textSubtle hover:bg-bgApp-hover'
                }`}
                onMouseEnter={() => setSelectedCommandIndex(i)}
                onClick={() => {
                  setInput(cmd.command + ' ');
                  setShowCommands(false);
                  inputRef.current?.focus();
                }}
              >
                <Slash className="w-3.5 h-3.5 opacity-50" />
                <div>
                  <span className="font-mono text-sm">{cmd.command}</span>
                  <span className="text-xs text-textSubtle ml-2">{cmd.description}</span>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Main prompt bar */}
      <div className="px-4 pb-3 pt-1 pointer-events-auto">
        <div className="max-w-2xl mx-auto">
          {/* Hint */}
          {config?.hint && !input && (
            <div className="flex justify-center mb-1.5">
              <span className="text-xs text-textSubtle">{config.hint}</span>
            </div>
          )}

          {/* Input bar */}
          <div className="relative flex items-center bg-bgApp-secondary border border-borderSubtle rounded-xl
            shadow-lg hover:border-borderStandard focus-within:border-borderStandard
            focus-within:ring-1 focus-within:ring-borderStandard/50 transition-all">
            <input
              ref={inputRef}
              type="text"
              value={input}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder={config?.placeholder}
              disabled={isLoading}
              className="flex-1 bg-transparent px-4 py-3 text-sm text-textStandard
                placeholder:text-textSubtle outline-none disabled:opacity-50"
            />

            {/* Keyboard shortcut hint */}
            {!input && (
              <div className="flex items-center gap-0.5 mr-2 text-textSubtle opacity-50">
                <Command className="w-3 h-3" />
                <span className="text-xs">K</span>
              </div>
            )}

            {/* Send button */}
            <button
              onClick={handleSubmit}
              disabled={!input.trim() || isLoading}
              className="mr-2 p-1.5 rounded-lg transition-all
                disabled:opacity-30 disabled:cursor-not-allowed
                text-textSubtle hover:text-textProminent hover:bg-bgApp-hover"
            >
              {isLoading ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Send className="w-4 h-4" />
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
