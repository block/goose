import { useState, useEffect, useRef } from 'react';
import { Button } from '../../ui/button';
import { KeyboardShortcuts } from '../../../utils/settingsTypes';
import { getShortcutLabel, formatShortcut } from './KeyboardShortcutsSection';

interface ShortcutRecorderProps {
  value: string;
  onSave: (shortcut: string) => void;
  onCancel: () => void;
  allShortcuts?: KeyboardShortcuts;
  currentKey?: keyof KeyboardShortcuts;
}

export function ShortcutRecorder({
  value,
  onSave,
  onCancel,
  allShortcuts,
  currentKey,
}: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(true);
  const [capturedShortcut, setCapturedShortcut] = useState(value);
  const [displayShortcut, setDisplayShortcut] = useState('');
  const [conflict, setConflict] = useState<string | null>(null);
  const inputRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  useEffect(() => {
    if (recording && inputRef.current) {
      inputRef.current.focus();
    }
  }, [recording]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!recording) return;

    e.preventDefault();
    e.stopPropagation();

    // Ignore modifier-only presses
    if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) {
      return;
    }

    const parts: string[] = [];

    if (e.ctrlKey || e.metaKey) {
      parts.push('CommandOrControl');
    }
    if (e.altKey) {
      parts.push('Alt');
    }
    if (e.shiftKey) {
      parts.push('Shift');
    }

    let key = e.key;

    const keyMap: Record<string, string> = {
      ' ': 'Space',
      ArrowUp: 'Up',
      ArrowDown: 'Down',
      ArrowLeft: 'Left',
      ArrowRight: 'Right',
      Escape: 'Esc',
      Delete: 'Delete',
      Backspace: 'Backspace',
      Tab: 'Tab',
      Enter: 'Return',
    };

    if (keyMap[key]) {
      key = keyMap[key];
    } else {
      key = key.length === 1 ? key.toUpperCase() : key;
    }

    parts.push(key);

    const accelerator = parts.join('+');
    setCapturedShortcut(accelerator);

    if (allShortcuts && currentKey) {
      const conflictingKey = Object.entries(allShortcuts).find(
        ([key, shortcut]) => key !== currentKey && shortcut === accelerator
      );
      if (conflictingKey) {
        setConflict(conflictingKey[0]);
      } else {
        setConflict(null);
      }
    }

    setDisplayShortcut(formatShortcut(accelerator));
    setRecording(false);
  };

  const handleStartRecording = () => {
    setRecording(true);
    setCapturedShortcut('');
    setDisplayShortcut('');
    setConflict(null);
  };

  const handleSave = () => {
    onSave(capturedShortcut);
  };

  const handleCancel = () => {
    onCancel();
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <div
          ref={inputRef}
          tabIndex={0}
          onKeyDown={handleKeyDown}
          onClick={handleStartRecording}
          className={`
            text-xs font-mono px-3 py-2 rounded border
            ${
              recording
                ? 'bg-background-default border-border-focus ring-1 ring-border-focus'
                : conflict
                  ? 'bg-background-muted border-yellow-600/50'
                  : 'bg-background-muted border-border-default cursor-pointer hover:border-border-focus'
            }
            focus:outline-none focus:ring-1 focus:ring-border-focus
            w-64 text-center
          `}
        >
          {recording ? (
            <span className="text-text-muted animate-pulse">Press shortcut...</span>
          ) : displayShortcut ? (
            <span className={conflict ? 'text-yellow-600' : 'text-text-default'}>
              {displayShortcut}
            </span>
          ) : capturedShortcut ? (
            <span className={conflict ? 'text-yellow-600' : 'text-text-default'}>
              {formatShortcut(capturedShortcut)}
            </span>
          ) : (
            <span className="text-text-muted">Click to record...</span>
          )}
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={handleSave}
          disabled={!capturedShortcut}
          className="text-xs"
        >
          Save
        </Button>
        <Button variant="secondary" size="sm" onClick={handleCancel} className="text-xs">
          Cancel
        </Button>
      </div>
      {conflict && (
        <div className="text-xs text-yellow-600 flex items-center gap-1">
          <span>⚠️</span>
          <span>
            This shortcut is already used by <strong>{getShortcutLabel(conflict)}</strong>. Saving
            will reassign it to this action.
          </span>
        </div>
      )}
    </div>
  );
}
