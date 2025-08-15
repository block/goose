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

    setIsSaving(true);
    try {
      // Update the environment variable via API
      const response = await fetch(getApiUrl('/config/env'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': await window.electron.getSecretKey(),
        },
        body: JSON.stringify({
          key: 'GOOSE_AUTO_COMPACT_THRESHOLD',
          value: (thresholdValue / 100).toString(),
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to update threshold');
      }

      setIsEditingThreshold(false);

      // Reload the page to reflect the new threshold
      window.location.reload();
    } catch (error) {
      console.error('Error saving threshold:', error);
      window.alert('Failed to save threshold. Please try again.');
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
          {alert.autoCompactThreshold !== undefined &&
            alert.autoCompactThreshold > 0 &&
            alert.autoCompactThreshold < 1 && (
              <div className="flex items-center justify-center gap-1 min-h-[20px]">
                {isEditingThreshold ? (
                  <>
                    <span className="text-[10px] opacity-70">Auto summarize at</span>
                    <input
                      type="number"
                      min="50"
                      max="95"
                      value={thresholdValue}
                      onChange={(e) => setThresholdValue(Number(e.target.value))}
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
                      className="w-12 px-1 text-[10px] bg-transparent border-b border-current outline-none text-center"
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
                      Auto summarize at {Math.round(alert.autoCompactThreshold * 100)}%
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

            {/* Percentage text positioned on the current dot */}
            {(() => {
              const progress = alert.progress!.current / alert.progress!.total;
              const progressPercentage = Math.round(progress * 100);

              // Calculate position - adjust for text width at high percentages
              let leftPosition = `${progress * 100}%`;
              let transform = 'translateX(-50%)';

              // Adjust position if text would be cut off
              if (progress > 0.92) {
                return (
                  <div
                    className="absolute -top-5 right-0 text-[10px] font-medium text-gray-600 dark:text-gray-400"
                    style={{ textAlign: 'right' as const }}
                  >
                    {progressPercentage}%
                  </div>
                );
              }

              return (
                <div
                  className="absolute -top-5 text-[10px] font-medium text-gray-600 dark:text-gray-400"
                  style={{
                    left: leftPosition,
                    transform,
                    textAlign: 'center' as const,
                  }}
                >
                  {progressPercentage}%
                </div>
              );
            })()}
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
          {alert.showSummarizeButton && alert.onSummarize && (
            <button
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                alert.onSummarize!();
              }}
              className="flex items-center gap-1.5 text-[11px] hover:opacity-80 cursor-pointer outline-none mt-1"
            >
              {alert.summarizeIcon}
              <span>Summarize now</span>
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
