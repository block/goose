import React, { useState } from 'react';
import { IoIosCloseCircle, IoIosWarning, IoIosInformationCircle } from 'react-icons/io';
import { FaPencilAlt, FaSave } from 'react-icons/fa';
import { cn } from '../../utils';
import { Alert, AlertType } from './types';
import { getApiUrl } from '../../config';

const alertIcons: Record<AlertType, React.ReactNode> = {
  [AlertType.Error]: <IoIosCloseCircle className="h-5 w-5" />,
  [AlertType.Warning]: <IoIosWarning className="h-5 w-5" />,
  [AlertType.Info]: <IoIosInformationCircle className="h-5 w-5" />,
};

interface AlertBoxProps {
  alert: Alert;
  className?: string;
  compactButtonEnabled?: boolean;
}

const alertStyles: Record<AlertType, string> = {
  [AlertType.Error]: 'bg-[#d7040e] text-white',
  [AlertType.Warning]: 'bg-[#cc4b03] text-white',
  [AlertType.Info]: 'dark:bg-white dark:text-black bg-black text-white',
};

export const AlertBox = ({ alert, className }: AlertBoxProps) => {
  const [isEditingThreshold, setIsEditingThreshold] = useState(false);
  const [thresholdValue, setThresholdValue] = useState(
    alert.autoCompactThreshold ? Math.round(alert.autoCompactThreshold * 100) : 80
  );
  const [isSaving, setIsSaving] = useState(false);

  const handleSaveThreshold = async () => {
    if (isSaving) return; // Prevent double-clicks

    // Validate threshold value - allow 0 and 100 as special values to disable
    const validThreshold = Math.max(0, Math.min(100, thresholdValue));
    if (validThreshold !== thresholdValue) {
      setThresholdValue(validThreshold);
    }

    setIsSaving(true);
    try {
      const newThreshold = validThreshold / 100; // Convert percentage to decimal
      console.log('Saving auto-compact threshold:', newThreshold);

      // Update the configuration via the upsert API
      const response = await fetch(getApiUrl('/config/upsert'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': await window.electron.getSecretKey(),
        },
        body: JSON.stringify({
          key: 'GOOSE_AUTO_COMPACT_THRESHOLD',
          value: newThreshold,
          is_secret: false,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Failed to update threshold: ${errorText}`);
      }

      const responseText = await response.text();
      console.log('Threshold save response:', responseText);

      setIsEditingThreshold(false);

      // Dispatch a custom event to notify other components that the threshold has changed
      // This allows ChatInput to reload the threshold without a page reload
      window.dispatchEvent(
        new CustomEvent('autoCompactThresholdChanged', {
          detail: { threshold: newThreshold },
        })
      );

      console.log('Dispatched autoCompactThresholdChanged event with threshold:', newThreshold);
    } catch (error) {
      console.error('Error saving threshold:', error);
      window.alert(
        `Failed to save threshold: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      className={cn('flex flex-col gap-2 px-3 py-3', alertStyles[alert.type], className)}
      onMouseDown={(e) => {
        // Prevent popover from closing when clicking inside the alert box
        if (isEditingThreshold) {
          e.stopPropagation();
        }
      }}
    >
      {alert.progress ? (
        <div className="flex flex-col gap-2">
          <span className="text-[11px]">{alert.message}</span>

          {/* Auto-compact threshold indicator with edit */}
          {alert.autoCompactThreshold !== undefined && (
            <div className="flex items-center justify-center gap-1 min-h-[20px]">
              {isEditingThreshold ? (
                <>
                  <span className="text-[10px] opacity-70">Auto summarize at</span>
                  <input
                    type="number"
                    min="0"
                    max="100"
                    step="1"
                    value={thresholdValue}
                    onChange={(e) => {
                      const val = parseInt(e.target.value, 10);
                      // Allow empty input for easier editing
                      if (e.target.value === '') {
                        setThresholdValue(0);
                      } else if (!isNaN(val)) {
                        // Clamp value between 0 and 100
                        setThresholdValue(Math.max(0, Math.min(100, val)));
                      }
                    }}
                    onBlur={(e) => {
                      // On blur, ensure we have a valid value
                      const val = parseInt(e.target.value, 10);
                      if (isNaN(val) || val < 0) {
                        setThresholdValue(0);
                      } else if (val > 100) {
                        setThresholdValue(100);
                      }
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        handleSaveThreshold();
                      } else if (e.key === 'Escape') {
                        setIsEditingThreshold(false);
                        setThresholdValue(
                          alert.autoCompactThreshold
                            ? Math.round(alert.autoCompactThreshold * 100)
                            : 80
                        );
                      }
                    }}
                    onFocus={(e) => {
                      // Select all text on focus for easier editing
                      e.target.select();
                    }}
                    onClick={(e) => {
                      // Prevent issues with text selection
                      e.stopPropagation();
                    }}
                    className="w-12 px-1 text-[10px] bg-white/10 border border-current/30 rounded outline-none text-center focus:bg-white/20 focus:border-current/50 transition-colors"
                    disabled={isSaving}
                    autoFocus
                  />
                  <span className="text-[10px] opacity-70">%</span>
                  <button
                    type="button"
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleSaveThreshold();
                    }}
                    disabled={isSaving}
                    className="p-1 hover:opacity-60 transition-opacity cursor-pointer relative z-50"
                    style={{ minWidth: '20px', minHeight: '20px', pointerEvents: 'auto' }}
                  >
                    <FaSave className="w-3 h-3" />
                  </button>
                </>
              ) : (
                <>
                  <span className="text-[10px] opacity-70">
                    {alert.autoCompactThreshold === 0 || alert.autoCompactThreshold === 1
                      ? 'Auto summarize disabled'
                      : `Auto summarize at ${Math.round(alert.autoCompactThreshold * 100)}%`}
                  </span>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setIsEditingThreshold(true);
                    }}
                    className="p-1 hover:opacity-60 transition-opacity cursor-pointer relative z-10"
                    style={{ minWidth: '20px', minHeight: '20px' }}
                  >
                    <FaPencilAlt className="w-3 h-3 opacity-70" />
                  </button>
                </>
              )}
            </div>
          )}

          <div className="flex justify-between w-full relative">
            {[...Array(30)].map((_, i) => {
              const progress = alert.progress!.current / alert.progress!.total;
              const progressPercentage = Math.round(progress * 100);
              const dotPosition = i / 29; // 0 to 1 range for 30 dots
              const isActive = dotPosition <= progress;
              const isThresholdDot =
                alert.autoCompactThreshold !== undefined &&
                alert.autoCompactThreshold > 0 &&
                alert.autoCompactThreshold < 1 &&
                Math.abs(dotPosition - alert.autoCompactThreshold) < 0.017; // ~1/30 tolerance

              // Determine the color based on progress percentage
              const getProgressColor = () => {
                if (progressPercentage <= 50) {
                  return 'bg-green-500'; // Green for 0-50%
                } else if (progressPercentage <= 75) {
                  return 'bg-yellow-500'; // Yellow for 51-75%
                } else if (progressPercentage <= 90) {
                  return 'bg-orange-500'; // Orange for 76-90%
                } else {
                  return 'bg-red-500'; // Red for 91-100%
                }
              };

              const progressColor = getProgressColor();
              const inactiveColor = 'bg-gray-300 dark:bg-gray-600';

              return (
                <div
                  key={i}
                  className={cn(
                    'rounded-full transition-all relative',
                    isThresholdDot
                      ? 'h-[6px] w-[6px] -mt-[2px]' // Make threshold dot twice as large
                      : 'h-[2px] w-[2px]',
                    isActive ? progressColor : inactiveColor
                  )}
                />
              );
            })}
          </div>
          <div className="flex justify-between items-baseline text-[11px]">
            <div className="flex gap-1 items-baseline">
              <span className={'dark:text-black/60 text-white/60'}>
                {alert.progress!.current >= 1000
                  ? (alert.progress!.current / 1000).toFixed(1) + 'k'
                  : alert.progress!.current}
              </span>
              <span className={'dark:text-black/40 text-white/40'}>
                {Math.round((alert.progress!.current / alert.progress!.total) * 100)}%
              </span>
            </div>
            <span className={'dark:text-black/60 text-white/60'}>
              {alert.progress!.total >= 1000
                ? (alert.progress!.total / 1000).toFixed(0) + 'k'
                : alert.progress!.total}
            </span>
          </div>
          {alert.showCompactButton && alert.onCompact && (
            <button
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                alert.onCompact!();
              }}
              disabled={alert.compactButtonDisabled}
              className={cn(
                'flex items-center gap-1.5 text-[11px] outline-none mt-1',
                alert.compactButtonDisabled
                  ? 'opacity-50 cursor-not-allowed'
                  : 'hover:opacity-80 cursor-pointer'
              )}
            >
              {alert.compactIcon}
              <span>Compact now</span>
            </button>
          )}
        </div>
      ) : (
        <>
          <div className="flex items-center gap-2">
            <div className="flex-shrink-0">{alertIcons[alert.type]}</div>
            <div className="flex flex-col gap-2 flex-1">
              <span className="text-[11px] break-words whitespace-pre-line">{alert.message}</span>
              {alert.action && (
                <a
                  role="button"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    alert.action?.onClick();
                  }}
                  className="text-[11px] text-left underline hover:opacity-80 cursor-pointer outline-none"
                >
                  {alert.action.text}
                </a>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
};
