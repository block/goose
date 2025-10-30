import { useState, useCallback, useEffect } from 'react';
import { Button } from '../../ui/button';
import { useToolOutputContext, formatHotkey } from '../../../contexts/ToolOutputContext';

export function HotkeySettings() {
  const { hotkey, setHotkey } = useToolOutputContext();
  const [isRecording, setIsRecording] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<string[]>([]);

  const handleRecordHotkey = useCallback(() => {
    setIsRecording(true);
    setRecordedKeys([]);
  }, []);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!isRecording) return;

      // Prevent default to avoid browser actions
      e.preventDefault();
      e.stopPropagation();

      const key = e.key.toLowerCase();

      // Don't record modifier keys alone
      if (['control', 'meta', 'alt', 'shift'].includes(key)) return;

      const keys = [];
      if (e.ctrlKey) keys.push('ctrl');
      if (e.metaKey) keys.push('meta');
      if (e.altKey) keys.push('alt');
      if (e.shiftKey) keys.push('shift');
      keys.push(key);

      setRecordedKeys(keys);
    },
    [isRecording]
  );

  const saveHotkey = useCallback(() => {
    if (recordedKeys.length === 0) return;

    const newHotkey = {
      key: recordedKeys[recordedKeys.length - 1],
      ctrl: recordedKeys.includes('ctrl'),
      meta: recordedKeys.includes('meta'),
      alt: recordedKeys.includes('alt'),
      shift: recordedKeys.includes('shift'),
    };

    setHotkey(newHotkey);
    setIsRecording(false);
    setRecordedKeys([]);
  }, [recordedKeys, setHotkey]);

  const cancelRecording = useCallback(() => {
    setIsRecording(false);
    setRecordedKeys([]);
  }, []);

  const resetToDefault = useCallback(() => {
    setHotkey({
      key: 'e',
      ctrl: true,
      meta: false,
      shift: false,
      alt: false,
    });
  }, [setHotkey]);

  useEffect(() => {
    if (!isRecording) return;

    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isRecording, handleKeyDown]);

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-medium mb-2">Tool Output Hotkey</h3>
        <p className="text-sm text-gray-600 mb-4">
          Configure a keyboard shortcut to expand/collapse all tool outputs in conversations.
        </p>
      </div>

      <div className="space-y-3">
        <div>
          <label className="block text-sm font-medium mb-2">Current Hotkey:</label>
          <div className="flex items-center space-x-2">
            <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-300 rounded">
              {formatHotkey(hotkey)}
            </kbd>
            <Button variant="outline" size="sm" onClick={handleRecordHotkey} disabled={isRecording}>
              {isRecording ? 'Recording...' : 'Change Hotkey'}
            </Button>
            <Button variant="outline" size="sm" onClick={resetToDefault} disabled={isRecording}>
              Reset to Default
            </Button>
          </div>
        </div>

        {isRecording && (
          <div className="p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-yellow-800">
                  Press your desired hotkey combination
                </p>
                {recordedKeys.length > 0 && (
                  <p className="text-sm text-yellow-700 mt-1">
                    Pressed:{' '}
                    {recordedKeys
                      .map((k) =>
                        k === 'ctrl'
                          ? 'Ctrl'
                          : k === 'meta'
                            ? 'Cmd'
                            : k === 'alt'
                              ? 'Alt'
                              : k === 'shift'
                                ? 'Shift'
                                : k.toUpperCase()
                      )
                      .join(' + ')}
                  </p>
                )}
              </div>
              <div className="flex space-x-2">
                <Button variant="outline" size="sm" onClick={cancelRecording}>
                  Cancel
                </Button>
                {recordedKeys.length > 0 && (
                  <Button variant="default" size="sm" onClick={saveHotkey}>
                    Save
                  </Button>
                )}
              </div>
            </div>
          </div>
        )}

        <div className="text-xs text-gray-500">
          <p>Default: Ctrl+E (Cmd+E on Mac)</p>
          <p>Hotkey is only active when hovering over tool arguments in conversations.</p>
        </div>
      </div>
    </div>
  );
}
