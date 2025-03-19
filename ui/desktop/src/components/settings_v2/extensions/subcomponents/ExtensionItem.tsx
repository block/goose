// ExtensionItem.tsx
import React from 'react';
import { Switch } from '../../../ui/switch';
import { Gear } from '../../../icons/Gear';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { getSubtitle, getFriendlyTitle } from './ExtensionList';

interface ExtensionItemProps {
  extension: FixedExtensionEntry;
  onToggle: (name: string) => void;
  onConfigure: (extension: FixedExtensionEntry) => void;
}

export default function ExtensionItem({ extension, onToggle, onConfigure }: ExtensionItemProps) {
  const renderFormattedSubtitle = () => {
    const subtitle = getSubtitle(extension);
    return subtitle.split('\n').map((part, index) => (
      <React.Fragment key={index}>
        {index === 0 ? part : <span className="font-mono text-xs">{part}</span>}
        {index < subtitle.split('\n').length - 1 && <br />}
      </React.Fragment>
    ));
  };
  return (
    <div className="rounded-lg border border-borderSubtle p-4 mb-2">
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-medium text-textStandard">{getFriendlyTitle(extension)}</h3>
        <div className="flex items-center gap-2">
          {/* Only show config button for non-builtin extensions */}
          {extension.type !== 'builtin' && (
            <button
              className="text-textSubtle hover:text-textStandard"
              onClick={() => onConfigure(extension)}
            >
              <Gear className="h-4 w-4" />
            </button>
          )}
          <Switch
            checked={extension.enabled}
            onCheckedChange={() => onToggle(extension.name)}
            variant="mono"
          />
        </div>
      </div>
      <p className="text-sm text-textSubtle">{renderFormattedSubtitle()}</p>
    </div>
  );
}
