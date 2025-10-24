import { forwardRef } from 'react';
import { Switch } from '../ui/switch';

export interface Extension {
  name: string;
  title: string;
  description: string;
}

interface ExtensionItemProps {
  extension: Extension;
  isEnabled: boolean;
  onToggle: (enabled: boolean) => void;
}

export const ExtensionItem = forwardRef<HTMLDivElement, ExtensionItemProps>(
  ({ extension, isEnabled, onToggle }, ref) => {
    const handleToggleClick = (e: React.MouseEvent) => {
      e.stopPropagation();
    };

    return (
      <div ref={ref} className="group text-sm">
        <div
          className={`flex items-center justify-between text-text-default py-2 px-2 ${
            isEnabled ? 'bg-background-muted' : 'bg-background-default hover:bg-background-muted'
          } rounded-lg transition-all`}
        >
          <div className="flex flex-1">
            <div className="flex flex-col">
              <h3 className="text-text-default">{extension.title}</h3>
              <p className="text-text-muted mt-[2px]">{extension.description}</p>
            </div>
          </div>

          <div className="flex items-center ml-4" onClick={handleToggleClick}>
            <Switch checked={isEnabled} onCheckedChange={onToggle} variant="mono" />
          </div>
        </div>
      </div>
    );
  }
);

ExtensionItem.displayName = 'ExtensionItem';
