import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Label } from '../../ui/label';
import { useConfig } from '../../ConfigContext';

export function HintsSection() {
  const { read, upsert } = useConfig();
  const [nestedEnabled, setNestedEnabled] = useState(false);

  useEffect(() => {
    const fetchCurrentSetting = async () => {
      try {
        const enabled = (await read('NESTED_GOOSE_HINTS', false)) as boolean;
        setNestedEnabled(enabled || false);
      } catch (error) {
        console.error('Error fetching nested hints setting:', error);
      }
    };

    fetchCurrentSetting();
  }, [read]);

  const handleToggle = async (enabled: boolean) => {
    try {
      await upsert('NESTED_GOOSE_HINTS', enabled, false);
      setNestedEnabled(enabled);
    } catch (error) {
      console.error('Failed to update nested hints setting:', error);
    }
  };
  return (
    <div className="space-y-4  ml-2">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <Label htmlFor="nested-hints" className="text-sm font-medium">
            Nested Hint Files Loading (eg: .goosehints)
          </Label>
          <p className="text-xs text-textSubtle mr-2">
            When enabled, loads hint files from current directory upwards to project root (.git) or
            filesystem root. <br />
            When disabled, only loads from current directory.
          </p>
        </div>
        <Switch
          id="nested-hints"
          checked={nestedEnabled}
          onCheckedChange={handleToggle}
          variant="mono"
        />
      </div>
    </div>
  );
}
