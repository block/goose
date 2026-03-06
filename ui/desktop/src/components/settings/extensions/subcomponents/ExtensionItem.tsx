import { useState, useEffect } from 'react';
import kebabCase from 'lodash/kebabCase';
import { Switch } from '../../../ui/switch';
import { Gear } from '../../../icons';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { Card, CardHeader, CardTitle, CardContent, CardAction } from '../../../ui/card';
import { useLocalization } from '../../../../contexts/LocalizationContext';

interface ExtensionItemProps {
  extension: FixedExtensionEntry;
  displayTitle: string;
  subtitleDescription: string | null;
  subtitleCommand: string | null;
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void> | void;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  isStatic?: boolean; // to not allow users to edit configuration
}

export default function ExtensionItem({
  extension,
  displayTitle,
  subtitleDescription,
  subtitleCommand,
  onToggle,
  onConfigure,
  isStatic,
}: ExtensionItemProps) {
  const { t } = useLocalization();
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
    } catch {
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
    return (
      <>
        {subtitleDescription && <span>{subtitleDescription}</span>}
        {subtitleDescription && subtitleCommand && <br />}
        {subtitleCommand && <span className="font-mono text-xs">{subtitleCommand}</span>}
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
      id={`extension-${kebabCase(extension.name)}`}
      className="transition-all duration-200 min-h-[120px] overflow-hidden"
    >
      <CardHeader>
        <CardTitle>{displayTitle}</CardTitle>

        <CardAction>
          <div className="flex items-center justify-end gap-2">
            {editable && (
              <button
                className="text-text-secondary hover:text-text-primary"
                aria-label={t('extensionsPage.configureExtension', { name: displayTitle })}
                onClick={() => onConfigure?.(extension)}
              >
                <Gear className="w-4 h-4" />
              </button>
            )}
            <Switch
              checked={(isToggling && visuallyEnabled) || extension.enabled}
              onCheckedChange={() => handleToggle(extension)}
              disabled={isToggling}
              variant="mono"
              aria-label={t('extensionsPage.toggleExtension', { name: displayTitle })}
            />
          </div>
        </CardAction>
      </CardHeader>
      <CardContent className="px-4 overflow-hidden text-sm break-words text-text-secondary">
        {renderSubtitle()}
      </CardContent>
    </Card>
  );
}
