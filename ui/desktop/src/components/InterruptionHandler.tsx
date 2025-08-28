import React, { useState, useEffect } from 'react';
import { AlertTriangle, StopCircle, PauseCircle, RotateCcw, Zap, AlertCircle } from 'lucide-react';
import { Button } from './ui/button';
import { InterruptionMatch } from '../utils/interruptionDetector';

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
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (match) {
      setIsVisible(true);
      if (match.keyword.action === 'redirect') {
        setShowRedirectInput(true);
      } else {
        setShowRedirectInput(false);
        setRedirectMessage('');
      }
    } else {
      setIsVisible(false);
    }
  }, [match]);

  if (!match) {
    return null;
  }

  const getIcon = () => {
    switch (match.keyword.action) {
      case 'stop':
        return <StopCircle className="w-6 h-6 text-red-500" />;
      case 'pause':
        return <PauseCircle className="w-6 h-6 text-amber-500" />;
      case 'redirect':
        return <RotateCcw className="w-6 h-6 text-blue-500" />;
      default:
        return <AlertTriangle className="w-6 h-6 text-orange-500" />;
    }
  };

  const getActionColor = () => {
    switch (match.keyword.action) {
      case 'stop':
        return {
          bg: 'bg-red-50 dark:bg-red-950/20',
          border: 'border-red-200 dark:border-red-800/50',
          text: 'text-red-800 dark:text-red-200',
          accent: 'text-red-600 dark:text-red-400',
        };
      case 'pause':
        return {
          bg: 'bg-amber-50 dark:bg-amber-950/20',
          border: 'border-amber-200 dark:border-amber-800/50',
          text: 'text-amber-800 dark:text-amber-200',
          accent: 'text-amber-600 dark:text-amber-400',
        };
      case 'redirect':
        return {
          bg: 'bg-blue-50 dark:bg-blue-950/20',
          border: 'border-blue-200 dark:border-blue-800/50',
          text: 'text-blue-800 dark:text-blue-200',
          accent: 'text-blue-600 dark:text-blue-400',
        };
      default:
        return {
          bg: 'bg-orange-50 dark:bg-orange-950/20',
          border: 'border-orange-200 dark:border-orange-800/50',
          text: 'text-orange-800 dark:text-orange-200',
          accent: 'text-orange-600 dark:text-orange-400',
        };
    }
  };

  const colors = getActionColor();

  const handleConfirm = () => {
    if (showRedirectInput && onRedirect && redirectMessage.trim()) {
      onRedirect(redirectMessage.trim());
    } else {
      onConfirmInterruption();
    }
  };

  const getActionTitle = () => {
    switch (match.keyword.action) {
      case 'stop':
        return 'Stop Processing';
      case 'pause':
        return 'Pause Processing';
      case 'redirect':
        return 'Redirect Processing';
      default:
        return 'Interrupt Processing';
    }
  };

  const getActionDescription = () => {
    switch (match.keyword.action) {
      case 'stop':
        return 'This will immediately stop the current processing and clear any queued messages.';
      case 'pause':
        return 'This will pause the current processing. Queued messages will be preserved.';
      case 'redirect':
        return 'This will stop current processing and redirect to a new task.';
      default:
        return 'This will interrupt the current processing.';
    }
  };

  return (
    <div
      className={`fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4 ${className}`}
    >
      <div
        className={`w-full max-w-md mx-auto transition-all duration-300 ease-out ${
          isVisible ? 'scale-100 opacity-100' : 'scale-95 opacity-0'
        }`}
      >
        {/* Main card */}
        <div
          className={`rounded-xl border shadow-2xl backdrop-blur-xl ${colors.bg} ${colors.border}`}
        >
          {/* Header */}
          <div className="p-6 border-b border-current/10">
            <div className="flex items-start gap-4">
              <div className="flex-shrink-0 p-2 rounded-full bg-white/50 dark:bg-black/20">
                {getIcon()}
              </div>
              <div className="flex-1">
                <h3 className={`text-lg font-semibold ${colors.text}`}>{getActionTitle()}</h3>
                <p className={`text-sm mt-1 ${colors.accent}`}>Detected: "{match.matchedText}"</p>
              </div>
              <div
                className={`text-xs px-2 py-1 rounded-full bg-white/30 dark:bg-black/20 ${colors.text}`}
              >
                {Math.round(match.confidence * 100)}% confident
              </div>
            </div>
          </div>

          {/* Content */}
          <div className="p-6">
            <div className="flex items-start gap-3 mb-4">
              <AlertCircle className={`w-4 h-4 mt-0.5 flex-shrink-0 ${colors.accent}`} />
              <p className={`text-sm leading-relaxed ${colors.text}`}>{getActionDescription()}</p>
            </div>

            {/* Redirect input */}
            {showRedirectInput && (
              <div className="mb-4 space-y-2">
                <label className={`text-sm font-medium ${colors.text}`}>
                  New task or instruction:
                </label>
                <textarea
                  value={redirectMessage}
                  onChange={(e) => setRedirectMessage(e.target.value)}
                  placeholder="Enter your new instruction..."
                  className={`w-full px-3 py-2 border rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-current/20 bg-white/50 dark:bg-black/20 ${colors.border} ${colors.text}`}
                  rows={3}
                  autoFocus
                />
              </div>
            )}

            {/* Confidence indicator */}
            <div className="mb-4">
              <div className="flex items-center justify-between text-xs mb-1">
                <span className={colors.accent}>Detection Confidence</span>
                <span className={colors.text}>{Math.round(match.confidence * 100)}%</span>
              </div>
              <div className="w-full bg-white/30 dark:bg-black/20 rounded-full h-2">
                <div
                  className={`h-2 rounded-full transition-all duration-500 ${
                    match.confidence > 0.8
                      ? 'bg-green-500'
                      : match.confidence > 0.6
                        ? 'bg-amber-500'
                        : 'bg-red-500'
                  }`}
                  style={{ width: `${match.confidence * 100}%` }}
                />
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="p-6 pt-0 flex gap-3">
            <Button
              variant="ghost"
              onClick={onCancelInterruption}
              className={`flex-1 hover:bg-white/20 dark:hover:bg-black/20 ${colors.text}`}
            >
              Cancel
            </Button>
            <Button
              onClick={handleConfirm}
              disabled={showRedirectInput && !redirectMessage.trim()}
              className={`flex-1 bg-white/80 hover:bg-white dark:bg-white/10 dark:hover:bg-white/20 ${colors.text} font-medium shadow-md hover:shadow-lg transition-all duration-200`}
            >
              <Zap className="w-4 h-4 mr-2" />
              {showRedirectInput
                ? 'Redirect'
                : match.keyword.action === 'stop'
                  ? 'Stop'
                  : 'Confirm'}
            </Button>
          </div>
        </div>

        {/* Backdrop hint */}
        <div className="text-center mt-4">
          <p className="text-xs text-white/60">
            Click outside or press Cancel to continue current processing
          </p>
        </div>
      </div>
    </div>
  );
};

export default InterruptionHandler;
