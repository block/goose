import React, { useState, useEffect } from 'react';
import { AlertTriangle, StopCircle, PauseCircle, RotateCcw } from 'lucide-react';
import { Button } from './ui/button';
import { InterruptionMatch, getInterruptionMessage } from '../utils/interruptionDetector';

interface InterruptionHandlerProps {
  match: InterruptionMatch | null;
  onConfirmInterruption: () => void;
  onCancelInterruption: () => void;
  onRedirect?: (newMessage: string) => void;
  className?: string;
}

export const InterruptionHandler: React.FC<InterruptionHandlerProps> = ({
  match,
  onConfirmInterruption,
  onCancelInterruption,
  onRedirect,
  className = '',
}) => {
  const [redirectMessage, setRedirectMessage] = useState('');
  const [showRedirectInput, setShowRedirectInput] = useState(false);

  useEffect(() => {
    if (match?.keyword.action === 'redirect') {
      setShowRedirectInput(true);
    } else {
      setShowRedirectInput(false);
      setRedirectMessage('');
    }
  }, [match]);

  if (!match) {
    return null;
  }

  const getIcon = () => {
    switch (match.keyword.action) {
      case 'stop':
        return <StopCircle className="w-5 h-5 text-red-500" />;
      case 'pause':
        return <PauseCircle className="w-5 h-5 text-yellow-500" />;
      case 'redirect':
        return <RotateCcw className="w-5 h-5 text-blue-500" />;
      default:
        return <AlertTriangle className="w-5 h-5 text-orange-500" />;
    }
  };

  const getActionColor = () => {
    switch (match.keyword.action) {
      case 'stop':
        return 'border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-950';
      case 'pause':
        return 'border-yellow-200 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950';
      case 'redirect':
        return 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950';
      default:
        return 'border-orange-200 bg-orange-50 dark:border-orange-800 dark:bg-orange-950';
    }
  };

  const handleRedirectSubmit = () => {
    if (redirectMessage.trim() && onRedirect) {
      onRedirect(redirectMessage.trim());
      setRedirectMessage('');
      setShowRedirectInput(false);
    }
  };

  return (
    <div className={`border rounded-lg p-4 ${getActionColor()} ${className}`}>
      <div className="flex items-start gap-3">
        {getIcon()}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-2">
            <h4 className="font-medium text-sm">
              Interruption Detected
            </h4>
            <span className="text-xs px-2 py-1 rounded-full bg-white/50 dark:bg-black/20">
              {Math.round(match.confidence * 100)}% confident
            </span>
          </div>
          
          <p className="text-sm text-muted-foreground mb-3">
            {getInterruptionMessage(match)}
          </p>

          {showRedirectInput && (
            <div className="mb-3">
              <input
                type="text"
                value={redirectMessage}
                onChange={(e) => setRedirectMessage(e.target.value)}
                placeholder="What would you like to do instead?"
                className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleRedirectSubmit();
                  }
                }}
                autoFocus
              />
            </div>
          )}

          <div className="flex gap-2">
            {match.keyword.action === 'redirect' && showRedirectInput ? (
              <>
                <Button
                  size="sm"
                  onClick={handleRedirectSubmit}
                  disabled={!redirectMessage.trim()}
                  className="text-xs"
                >
                  Redirect
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setShowRedirectInput(false);
                    setRedirectMessage('');
                    onCancelInterruption();
                  }}
                  className="text-xs"
                >
                  Cancel
                </Button>
              </>
            ) : (
              <>
                <Button
                  size="sm"
                  onClick={onConfirmInterruption}
                  className="text-xs"
                >
                  Yes, {match.keyword.action}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={onCancelInterruption}
                  className="text-xs"
                >
                  Continue processing
                </Button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default InterruptionHandler;
