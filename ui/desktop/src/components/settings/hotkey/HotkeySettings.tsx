import { useState, useEffect } from 'react';
import { useToolOutputContext, formatHotkey } from '../../../contexts/ToolOutputContext';
import { Key, Keyboard } from 'lucide-react';

// Ensure imports are used to avoid TypeScript errors
const _ensureImportsUsed = { formatHotkey, Key, Keyboard };

export default function HotkeySettings() {
  const { hotkey, setHotkey, isHotkeyActive } = useToolOutputContext();
  const [isRecording, setIsRecording] = useState(false);
  const [tempHotkey, setTempHotkey] = useState(hotkey);

  useEffect(() => {
    setTempHotkey(hotkey);
  }, [hotkey]);

  const handleRecordHotkey = () => {
    setIsRecording(true);

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();

      const newHotkey = {
        key: e.key.toLowerCase(),
        ctrl: e.ctrlKey,
        meta: e.metaKey,
        shift: e.shiftKey,
        alt: e.altKey,
      };

      // Don't allow empty modifier combinations for single letters
      if (newHotkey.key.length === 1 && !newHotkey.ctrl && !newHotkey.meta && !newHotkey.alt) {
        return;
      }

      setTempHotkey(newHotkey);
      setIsRecording(false);
    };

    const handleBlur = () => {
      setIsRecording(false);
    };

    document.addEventListener('keydown', handleKeyDown);
    window.addEventListener('blur', handleBlur);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('blur', handleBlur);
    };
  };

  const handleSaveHotkey = async () => {
    await setHotkey(tempHotkey);
  };

  const handleResetHotkey = () => {
    setTempHotkey({
      key: 'e',
      ctrl: true,
      meta: false,
      shift: false,
      alt: false,
    });
  };

  const toggleModifier = (modifier: 'ctrl' | 'meta' | 'shift' | 'alt') => {
    setTempHotkey(prev => ({
      ...prev,
      [modifier]: !prev[modifier],
    }));
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center space-x-2">
        <div className="flex items-center space-x-1">
          <Keyboard className="h-4 w-4" />
          <span className="text-sm font-medium">Current Hotkey:</span>
        </div>
        <div className="px-3 py-1 bg-muted rounded text-sm font-mono">
          {formatHotkey(hotkey)}
        </div>
        <div className="flex items-center space-x-1">
          <div className={`w-2 h-2 rounded-full ${isHotkeyActive ? 'bg-green-500' : 'bg-gray-400'}`} />
          <span className="text-xs text-muted-foreground">
            {isHotkeyActive ? 'Active' : 'Inactive'}
          </span>
        </div>
      </div>

      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium">Record New Hotkey</label>
          <div className="flex items-center space-x-2 mt-1">
            <button
              className={`inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm transition-all cursor-pointer disabled:pointer-events-none disabled:opacity-50 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[1px] px-3 py-1 rounded-md text-sm ${
                isRecording
                  ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90'
                  : 'bg-background border border-border hover:bg-accent hover:text-accent-foreground'
              }`}
              onClick={handleRecordHotkey}
            >
              <Key className="h-3 w-3" />
              <span>{isRecording ? 'Recording...' : 'Record'}</span>
            </button>
            <div className="px-3 py-1 bg-muted rounded text-sm font-mono min-w-[120px]">
              {formatHotkey(tempHotkey)}
            </div>
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            Click record and press your desired key combination
          </p>
        </div>

        <div>
          <label className="text-sm font-medium">Modifiers</label>
          <div className="flex items-center space-x-4 mt-2">
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="ctrl"
                checked={tempHotkey.ctrl}
                onChange={() => toggleModifier('ctrl')}
                className="rounded border-borderStandard"
              />
              <label htmlFor="ctrl" className="text-sm">Ctrl</label>
            </div>
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="meta"
                checked={tempHotkey.meta}
                onChange={() => toggleModifier('meta')}
                className="rounded border-borderStandard"
              />
              <label htmlFor="meta" className="text-sm">Cmd</label>
            </div>
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="alt"
                checked={tempHotkey.alt}
                onChange={() => toggleModifier('alt')}
                className="rounded border-borderStandard"
              />
              <label htmlFor="alt" className="text-sm">Alt</label>
            </div>
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="shift"
                checked={tempHotkey.shift}
                onChange={() => toggleModifier('shift')}
                className="rounded border-borderStandard"
              />
              <label htmlFor="shift" className="text-sm">Shift</label>
            </div>
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <label className="text-sm font-medium">Key:</label>
          <input
            type="text"
            value={tempHotkey.key}
            onChange={(e) => setTempHotkey(prev => ({
              ...prev,
              key: e.target.value.toLowerCase()
            }))}
            className="w-20 text-center font-mono px-2 py-1 border border-border rounded text-sm"
            maxLength={1}
          />
        </div>

        <div className="flex items-center space-x-2">
          <button
            className="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm transition-all cursor-pointer disabled:pointer-events-none disabled:opacity-50 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[1px] px-3 py-1 rounded-md text-sm bg-background border border-border hover:bg-accent hover:text-accent-foreground"
            onClick={handleResetHotkey}
          >
            Reset to Default
          </button>
          <button
            className="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm transition-all cursor-pointer disabled:pointer-events-none disabled:opacity-50 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[1px] px-3 py-1 rounded-md text-sm bg-primary text-primary-foreground hover:bg-primary/90"
            onClick={handleSaveHotkey}
            disabled={JSON.stringify(tempHotkey) === JSON.stringify(hotkey)}
          >
            Save Hotkey
          </button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground p-3 bg-muted rounded">
        <p className="font-medium mb-1">About this feature:</p>
        <ul className="space-y-1">
          <li>• This hotkey expands all truncated tool outputs in conversation</li>
          <li>• The hotkey only works when hovering over tool arguments</li>
          <li>• Default: Ctrl+E (Cmd+E on Mac)</li>
          <li>• Avoid conflicts with browser shortcuts like Ctrl+R</li>
        </ul>
      </div>
    </div>
  );
}