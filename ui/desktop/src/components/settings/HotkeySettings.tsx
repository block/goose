import { useState, useEffect } from 'react';
import {
  useToggleToolOutputContext,
  HotkeyConfig,
  formatHotkey,
} from '../../hooks/useToggleToolOutput';

interface HotkeySettingsProps {
  onClose?: () => void;
}

const hotkeyOptions = [
  { key: 'e', label: 'E' },
  { key: 't', label: 'T' },
  { key: 'o', label: 'O' },
  { key: 'f', label: 'F' },
  { key: 'g', label: 'G' },
  { key: 'h', label: 'H' },
  { key: 'r', label: 'R' },
  { key: 's', label: 'S' },
  { key: 'l', label: 'L' },
];

export function HotkeySettings({ onClose }: HotkeySettingsProps) {
  const { hotkey, setHotkey } = useToggleToolOutputContext();
  const [tempHotkey, setTempHotkey] = useState<HotkeyConfig>(hotkey);
  const [isRecording, setIsRecording] = useState(false);
  const [recordingTimeout, setRecordingTimeout] = useState<number | null>(null);

  // Update temp hotkey when hotkey changes
  useEffect(() => {
    setTempHotkey(hotkey);
  }, [hotkey]);

  const handleRecordHotkey = () => {
    setIsRecording(true);

    // Clear any existing timeout
    if (recordingTimeout) {
      window.clearTimeout(recordingTimeout);
    }

    // Set timeout to stop recording after 5 seconds
    const timeout = window.setTimeout(() => {
      setIsRecording(false);
    }, 5000);

    setRecordingTimeout(timeout);

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();

      // Don't record special keys alone (Ctrl, Alt, Shift, Meta)
      if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) {
        return;
      }

      const newHotkey: HotkeyConfig = {
        key: e.key.toLowerCase(),
        ctrl: e.ctrlKey,
        meta: e.metaKey,
        shift: e.shiftKey,
        alt: e.altKey,
      };

      setTempHotkey(newHotkey);
      setIsRecording(false);

      if (recordingTimeout) {
        window.clearTimeout(recordingTimeout);
      }

      document.removeEventListener('keydown', handleKeyDown);
    };

    document.addEventListener('keydown', handleKeyDown);

    // Cleanup on unmount
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      if (recordingTimeout) {
        window.clearTimeout(recordingTimeout);
      }
    };
  };

  const handlePresetHotkey = (key: string) => {
    setTempHotkey({
      key,
      ctrl: true,
      meta: false,
      shift: false,
      alt: false,
    });
  };

  const handleSave = () => {
    setHotkey(tempHotkey);
    if (onClose) {
      onClose();
    }
  };

  const handleReset = () => {
    setTempHotkey({
      key: 'e',
      ctrl: true,
      meta: false,
      shift: false,
      alt: false,
    });
  };

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">Tool Output Hotkey Settings</h3>
        <p className="text-sm text-textSubtle mb-6">
          Customize the hotkey used to expand all truncated tool outputs at once.
        </p>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-text mb-2">Current Hotkey</label>
          <div className="flex items-center space-x-3">
            <div className="px-4 py-2 bg-background-secondary rounded font-mono text-sm">
              {formatHotkey(tempHotkey)}
            </div>
            <button
              onClick={handleRecordHotkey}
              className={`px-4 py-2 rounded text-sm font-medium ${
                isRecording
                  ? 'bg-background-inverse text-text-on-accent'
                  : 'bg-background-hover text-text hover:bg-background-secondary'
              }`}
            >
              {isRecording ? 'Recording...' : 'Record New Hotkey'}
            </button>
          </div>
          {isRecording && (
            <p className="text-xs text-textSubtle mt-2">
              Press the key combination you want to use. Recording will stop automatically after 5
              seconds.
            </p>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium text-text mb-2">Quick Presets</label>
          <div className="grid grid-cols-3 gap-2">
            {hotkeyOptions.map((option) => (
              <button
                key={option.key}
                onClick={() => handlePresetHotkey(option.key)}
                className={`px-3 py-2 rounded text-sm ${
                  tempHotkey.key === option.key &&
                  tempHotkey.ctrl &&
                  !tempHotkey.alt &&
                  !tempHotkey.shift
                    ? 'bg-accent text-text-on-accent'
                    : 'bg-background-hover text-text hover:bg-background-secondary'
                }`}
              >
                Ctrl+{option.label.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-text mb-2">Preview</label>
          <div className="p-4 bg-background-secondary rounded">
            <p className="text-sm text-textSubtle">
              Click to expand (or press {formatHotkey(tempHotkey)} to expand all)
            </p>
          </div>
        </div>
      </div>

      <div className="flex justify-between items-center pt-4 border-t border-border">
        <button onClick={handleReset} className="px-4 py-2 text-sm text-textSubtle hover:text-text">
          Reset to Default
        </button>
        <div className="space-x-3">
          {onClose && (
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm text-text hover:bg-background-hover rounded"
            >
              Cancel
            </button>
          )}
          <button
            onClick={handleSave}
            className="px-4 py-2 text-sm bg-accent text-text-on-accent rounded hover:bg-accent-hover"
          >
            Save Changes
          </button>
        </div>
      </div>
    </div>
  );
}
