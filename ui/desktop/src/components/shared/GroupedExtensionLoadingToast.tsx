import { ChevronDown, ChevronUp, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { useNavigation } from '../../hooks/useNavigation';
import { startNewSession } from '../../sessions';
import { formatExtensionErrorMessage } from '../../utils/extensionErrorUtils';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { formatExtensionName } from '../settings/extensions/subcomponents/ExtensionList';
import { Button } from '../ui/atoms/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/molecules/collapsible';

export interface ExtensionLoadingStatus {
  name: string;
  status: 'loading' | 'success' | 'error';
  error?: string;
  recoverHints?: string;
}

interface ExtensionLoadingToastProps {
  extensions: ExtensionLoadingStatus[];
  totalCount: number;
  isComplete: boolean;
}

export function GroupedExtensionLoadingToast({
  extensions,
  totalCount,
  isComplete,
}: ExtensionLoadingToastProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [copiedExtension, setCopiedExtension] = useState<string | null>(null);
  const setView = useNavigation();

  const successCount = extensions.filter((ext) => ext.status === 'success').length;
  const errorCount = extensions.filter((ext) => ext.status === 'error').length;

  const getStatusIcon = (status: 'loading' | 'success' | 'error') => {
    switch (status) {
      case 'loading':
        return <Loader2 className="w-4 h-4 animate-spin text-blue-500" />;
      case 'success':
        return <div className="w-4 h-4 rounded-full bg-green-500" />;
      case 'error':
        return <div className="w-4 h-4 rounded-full bg-red-500" />;
    }
  };

  const getSummaryText = () => {
    if (!isComplete) {
      return `Loading ${totalCount} extension${totalCount !== 1 ? 's' : ''}...`;
    }

    if (errorCount === 0) {
      return `Successfully loaded ${successCount} extension${successCount !== 1 ? 's' : ''}`;
    }

    return `Loaded ${successCount}/${totalCount} extension${totalCount !== 1 ? 's' : ''}`;
  };

  const getSummaryIcon = () => {
    if (!isComplete) {
      return <Loader2 className="w-5 h-5 animate-spin text-blue-500" />;
    }

    if (errorCount === 0) {
      return <div className="w-5 h-5 rounded-full bg-green-500" />;
    }

    return <div className="w-5 h-5 rounded-full bg-yellow-500" />;
  };

  return (
    <div className="w-full">
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <div className="flex flex-col">
          {/* Main summary section - clickable */}
          <CollapsibleTrigger asChild>
            <div className="flex items-start gap-3 pr-8 cursor-pointer hover:opacity-90 transition-opacity">
              <div className="flex items-center gap-3 flex-1 min-w-0">
                {getSummaryIcon()}
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-base">{getSummaryText()}</div>
                  {errorCount > 0 && (
                    <div className="text-sm opacity-90">
                      {errorCount} extension{errorCount !== 1 ? 's' : ''} failed to load
                    </div>
                  )}
                </div>
              </div>
            </div>
          </CollapsibleTrigger>

          {/* Expanded details section */}
          <CollapsibleContent className="overflow-hidden">
            <div className="mt-3 pt-3 border-t border-white/20">
              <div className="space-y-3 max-h-64 overflow-y-auto pr-2 pl-1">
                {extensions.map((ext) => {
                  const friendlyName = formatExtensionName(ext.name);

                  return (
                    <div key={ext.name} className="flex flex-col gap-2">
                      <div className="flex items-center gap-3 text-sm">
                        {getStatusIcon(ext.status)}
                        <div className="flex-1 min-w-0 truncate">{friendlyName}</div>
                      </div>
                      {ext.status === 'error' && ext.error && (
                        <div className="ml-7 flex flex-col gap-2">
                          <div className="text-xs opacity-75 break-words">
                            {formatExtensionErrorMessage(ext.error, 'Failed to add extension')}
                          </div>
                          <div className="flex gap-2">
                            {ext.recoverHints && setView && (
                              <Button
                                size="sm"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  startNewSession(
                                    ext.recoverHints,
                                    setView,
                                    getInitialWorkingDir()
                                  );
                                }}
                              >
                                Ask goose
                              </Button>
                            )}
                            <Button
                              size="sm"
                              onClick={(e) => {
                                e.stopPropagation();
                                navigator.clipboard.writeText(ext.error!);
                                setCopiedExtension(ext.name);
                                setTimeout(() => setCopiedExtension(null), 2000);
                              }}
                            >
                              {copiedExtension === ext.name ? 'Copied!' : 'Copy error'}
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </CollapsibleContent>

          {/* Toggle button */}
          {totalCount > 0 && (
            <CollapsibleTrigger asChild>
              <button
                className="flex items-center justify-center gap-1 text-xs opacity-60 hover:opacity-100 transition-opacity mt-2 py-1.5 w-full"
                aria-label={isOpen ? 'Collapse details' : 'Expand details'}
              >
                {isOpen ? (
                  <>
                    <span>Show less</span>
                    <ChevronUp className="w-3 h-3" />
                  </>
                ) : (
                  <>
                    <span>Show details</span>
                    <ChevronDown className="w-3 h-3" />
                  </>
                )}
              </button>
            </CollapsibleTrigger>
          )}
        </div>
      </Collapsible>
    </div>
  );
}
