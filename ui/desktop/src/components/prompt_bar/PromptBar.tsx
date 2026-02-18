import { useState, useRef, useEffect, useCallback } from 'react';
import { Loader2, Command, Slash, Mic, MicOff, Paperclip, X, FileIcon } from 'lucide-react';
import { useUnifiedInput, type SlashCommand } from '../../contexts/UnifiedInputContext';
import { useAudioRecorder } from '../../hooks/useAudioRecorder';
import { useFileDrop, type DroppedFile } from '../../hooks/useFileDrop';
import Send from '../icons/Send';

export default function PromptBar() {
  const ctx = useUnifiedInput();
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showCommands, setShowCommands] = useState(false);
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const isHidden = !ctx.showPromptBar;

  const config = ctx.config;
  const slashCommands = ctx.slashCommands;
  const submitPrompt = ctx.submitPrompt;

  // ─── File Drop ─────────────────────────────────────────────────────
  const {
    droppedFiles,
    setDroppedFiles,
    handleDrop,
    handleDragOver,
  } = useFileDrop();

  const handleRemoveFile = (id: string) => {
    setDroppedFiles((prev: DroppedFile[]) => prev.filter((f: DroppedFile) => f.id !== id));
  };

  const handleFilePickerClick = () => {
    fileInputRef.current?.click();
  };

  const handleFilePickerChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    // Create DroppedFile objects from selected files
    const newFiles: DroppedFile[] = Array.from(files).map((file) => ({
      id: `file-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      path: window.electron?.getPathForFile?.(file) || file.name,
      name: file.name,
      type: file.type,
      isImage: file.type.startsWith('image/'),
    }));
    setDroppedFiles((prev: DroppedFile[]) => [...prev, ...newFiles]);
    // Reset input so same file can be re-selected
    e.target.value = '';
  };

  // ─── Voice Dictation ───────────────────────────────────────────────
  const {
    isRecording,
    isTranscribing,
    startRecording,
    stopRecording,
  } = useAudioRecorder({
    onTranscription: (text: string) => {
      setInput((prev) => (prev ? prev + ' ' + text : text));
      textareaRef.current?.focus();
    },
    onError: (message: string) => {
      console.error('[PromptBar] Voice dictation error:', message);
    },
  });

  const handleMicClick = () => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  };

  // ─── Slash Commands ────────────────────────────────────────────────
  const filteredCommands: SlashCommand[] = input.startsWith('/')
    ? slashCommands.filter((cmd) =>
        cmd.command.toLowerCase().startsWith(input.toLowerCase())
      )
    : [];

  // ─── Submit ────────────────────────────────────────────────────────
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

    // Submit as new session prompt
    setIsLoading(true);
    try {
      submitPrompt(trimmed);
      setInput('');
      setDroppedFiles([]);
    } catch (error) {
      console.error('Failed to submit from prompt bar:', error);
    } finally {
      setIsLoading(false);
    }
  }, [input, isLoading, slashCommands, submitPrompt, setDroppedFiles]);

  // ─── Input Handlers ────────────────────────────────────────────────
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    if (e.target.value.startsWith('/')) {
      setShowCommands(true);
      setSelectedCommandIndex(0);
    } else {
      setShowCommands(false);
    }
    // Auto-resize textarea
    const textarea = e.target;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 150) + 'px';
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (showCommands && filteredCommands.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedCommandIndex((i) => (i + 1) % filteredCommands.length);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedCommandIndex((i) => (i - 1 + filteredCommands.length) % filteredCommands.length);
      } else if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        const cmd = filteredCommands[selectedCommandIndex];
        if (cmd) {
          setInput(cmd.command + ' ');
          setShowCommands(false);
        }
      } else if (e.key === 'Escape') {
        setShowCommands(false);
      }
    } else if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // ─── Global Cmd/Ctrl+K ────────────────────────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        textareaRef.current?.focus();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  if (isHidden) return null;

  const hasFiles = droppedFiles.length > 0;

  return (
    <div
      className="fixed bottom-0 left-[var(--sidebar-width,0px)] right-0 z-50 pointer-events-none"
      onDrop={handleDrop}
      onDragOver={handleDragOver}
    >
      {/* Command palette dropdown */}
      {showCommands && filteredCommands.length > 0 && (
        <div className="mx-4 mb-1 pointer-events-auto">
          <div className="bg-background-default-secondary border border-border-default rounded-lg shadow-lg overflow-hidden max-w-2xl mx-auto">
            {filteredCommands.map((cmd: SlashCommand, i: number) => (
              <button
                key={cmd.command}
                className={`w-full px-4 py-2.5 flex items-center gap-3 text-left transition-colors ${
                  i === selectedCommandIndex
                    ? 'bg-background-default-active text-text-default font-semibold'
                    : 'text-text-muted hover:bg-background-default-hover'
                }`}
                onMouseEnter={() => setSelectedCommandIndex(i)}
                onClick={() => {
                  setInput(cmd.command + ' ');
                  setShowCommands(false);
                  textareaRef.current?.focus();
                }}
              >
                <Slash className="w-3.5 h-3.5 opacity-50" />
                <div>
                  <span className="font-mono text-sm">{cmd.command}</span>
                  <span className="text-xs text-text-muted ml-2">{cmd.description}</span>
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
          {config?.hint && !input && !hasFiles && (
            <div className="flex justify-center mb-1.5">
              <span className="text-xs text-text-muted">{config.hint}</span>
            </div>
          )}

          {/* Dropped files preview */}
          {hasFiles && (
            <div className="flex flex-wrap gap-2 mb-2">
              {droppedFiles.map((file: DroppedFile) => (
                <div
                  key={file.id}
                  className="flex items-center gap-1.5 bg-background-default-secondary border border-border-default rounded-lg px-2.5 py-1.5 text-xs"
                >
                  <FileIcon className="w-3 h-3 text-text-muted" />
                  <span className="text-text-default truncate max-w-[150px]">{file.name}</span>
                  <button
                    onClick={() => handleRemoveFile(file.id)}
                    className="text-text-muted hover:text-text-default font-semibold transition-colors"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Voice recording indicator */}
          {(isRecording || isTranscribing) && (
            <div className="flex items-center justify-center gap-2 mb-2">
              <div className={`w-2 h-2 rounded-full ${isRecording ? 'bg-red-500 animate-pulse' : 'bg-amber-500 animate-pulse'}`} />
              <span className="text-xs text-text-muted">
                {isRecording ? 'Recording…' : 'Transcribing…'}
              </span>
            </div>
          )}

          {/* Input bar */}
          <div className="relative flex items-end bg-background-default-secondary border border-border-default rounded-xl
            shadow-lg hover:border-border-accent focus-within:border-border-accent
            focus-within:ring-1 focus-within:ring-border-accent/50 transition-all">

            {/* File attach button */}
            <button
              onClick={handleFilePickerClick}
              className="ml-2 mb-2.5 p-1.5 rounded-lg transition-all text-text-muted hover:text-text-default font-semibold hover:bg-background-default-hover"
              title="Attach files"
            >
              <Paperclip className="w-4 h-4" />
            </button>
            <input
              ref={fileInputRef}
              type="file"
              multiple
              className="hidden"
              onChange={handleFilePickerChange}
            />

            {/* Textarea */}
            <textarea
              ref={textareaRef}
              value={input}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder={config?.placeholder}
              disabled={isLoading}
              rows={1}
              className="flex-1 bg-transparent px-3 py-3 text-sm text-text-default
                placeholder:text-text-muted outline-none disabled:opacity-50
                resize-none overflow-y-auto max-h-[150px]"
            />

            {/* Keyboard shortcut hint */}
            {!input && !hasFiles && (
              <div className="flex items-center gap-0.5 mb-3 mr-1 text-text-muted opacity-50">
                <Command className="w-3 h-3" />
                <span className="text-xs">K</span>
              </div>
            )}

            {/* Mic button */}
            <button
              onClick={handleMicClick}
              disabled={isTranscribing}
              className={`mb-2.5 p-1.5 rounded-lg transition-all
                ${isRecording
                  ? 'text-red-500 bg-red-500/10 hover:bg-red-500/20'
                  : 'text-text-muted hover:text-text-default font-semibold hover:bg-background-default-hover'}
                disabled:opacity-30 disabled:cursor-not-allowed`}
              title={isRecording ? 'Stop recording' : 'Voice input'}
            >
              {isRecording ? <MicOff className="w-4 h-4" /> : <Mic className="w-4 h-4" />}
            </button>

            {/* Send button */}
            <button
              onClick={handleSubmit}
              disabled={!input.trim() || isLoading}
              className="mr-2 mb-2.5 p-1.5 rounded-lg transition-all
                disabled:opacity-30 disabled:cursor-not-allowed
                text-text-muted hover:text-text-default font-semibold hover:bg-background-default-hover"
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
