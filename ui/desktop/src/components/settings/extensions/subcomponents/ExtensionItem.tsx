import { useState, useEffect } from 'react';
import { Switch } from '../../../ui/switch';
import { Gear } from '../../../icons/Gear';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { getSubtitle, getFriendlyTitle } from './ExtensionList';
import { Card, CardHeader, CardTitle, CardContent, CardAction } from '../../../ui/card';

interface ExtensionItemProps {
  extension: FixedExtensionEntry;
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void> | void;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  isStatic?: boolean; // to not allow users to edit configuration
}

export default function ExtensionItem({
  extension,
  onToggle,
  onConfigure,
  isStatic,
}: ExtensionItemProps) {
  // Add local state to track the visual toggle state
  const [visuallyEnabled, setVisuallyEnabled] = useState(extension.enabled);
  // Track if we're in the process of toggling
  const [isToggling, setIsToggling] = useState(false);

  const handleToggle = async (ext: FixedExtensionEntry) => {
    // Prevent multiple toggles while one is in progress
    if (isToggling) return;

    setIsToggling(true);

    // Immediately update visual state
    const newState = !ext.enabled;
    setVisuallyEnabled(newState);

    try {
      // Call the actual toggle function that performs the async operation
      await onToggle(ext);
      // Success case is handled by the useEffect below when extension.enabled changes
    } catch (error) {
      // If there was an error, revert the visual state
      console.log('Toggle failed, reverting visual state');
      setVisuallyEnabled(!newState);
    } finally {
      setIsToggling(false);
    }
  };

  // Update visual state when the actual extension state changes
  useEffect(() => {
    if (!isToggling) {
      setVisuallyEnabled(extension.enabled);
    }
  }, [extension.enabled, isToggling]);

  const renderSubtitle = () => {
    const { description, command } = getSubtitle(extension);
    return (
      <>
        {description && <span>{description}</span>}
        {description && command && <br />}
        {command && <span className="font-mono text-xs">{command}</span>}
      </>
    );
  };

  // Bundled extensions and builtins are not editable
  // Over time we can take the first part of the conditional away as people have bundled: true in their config.yaml entries

  // allow configuration editing if extension is not a builtin/bundled extension AND isStatic = false
  const editable =
    !(extension.type === 'builtin' || ('bundled' in extension && extension.bundled)) && !isStatic;

  return (
    <Card
      className="transition-all duration-200 hover:shadow-default hover:cursor-pointer min-h-[120px] overflow-hidden"
      onClick={() => handleToggle(extension)}
    >
      <CardHeader>
        <CardTitle className="">{getFriendlyTitle(extension)}</CardTitle>

        <CardAction onClick={(e) => e.stopPropagation()}>
          <div className="flex items-center justify-end gap-2">
            {editable && (
              <button
                className="text-textSubtle hover:text-textStandard"
                onClick={() => (onConfigure ? onConfigure(extension) : () => {})}
              >
                <Gear className="h-4 w-4" />
              </button>
            )}
            <Switch
              checked={(isToggling && visuallyEnabled) || extension.enabled}
              onCheckedChange={() => handleToggle(extension)}
              disabled={isToggling}
              variant="mono"
            />
          </div>
        </CardAction>
      </CardHeader>
      <CardContent className="px-4 text-sm text-text-muted overflow-hidden break-words">
        {renderSubtitle()}
      </CardContent>
    </Card>
  );
}
