import React, { useRef, useEffect } from 'react';
import { IoIosCloseCircle, IoIosWarning } from 'react-icons/io';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { cn } from '../../utils';
import { Alert, AlertType } from './alerts';

//todo: for some reason clearTimeout is not available globally but setTimeout is on other files?
const { clearTimeout } = window;

const alertIcons: Record<AlertType, React.ReactNode> = {
  [AlertType.Error]: <IoIosCloseCircle className="h-5 w-5" />,
  [AlertType.Warning]: <IoIosWarning className="h-5 w-5" />,
};

const alertStyles: Record<AlertType, string> = {
  [AlertType.Error]: 'bg-[#d7040e] text-white',
  [AlertType.Warning]: 'bg-[#cc4b03] text-white',
};

interface AlertPopoverProps {
  alerts: Alert[];
}

export default function BottomMenuAlertPopover({ alerts }: AlertPopoverProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [hasShownInitial, setHasShownInitial] = React.useState(false);
  const previousAlertsRef = useRef<Alert[]>([]);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const popoverRef = useRef<HTMLDivElement>(null);

  // Function to start the hide timer
  const startHideTimer = (duration = 3000) => {
    console.log('Starting hide timer');
    // Clear any existing timer
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    // Start new timer
    hideTimerRef.current = setTimeout(() => {
      console.log('Hide timer completed, closing popover');
      setIsOpen(false);
    }, duration);
  };

  // Handle initial show and new alerts
  useEffect(() => {
    if (alerts.length === 0) return;

    const hasNewAlerts = alerts.length > previousAlertsRef.current.length;
    previousAlertsRef.current = alerts;

    if (!hasShownInitial || hasNewAlerts) {
      console.log('Auto-showing popover');
      setIsOpen(true);
      setHasShownInitial(true);
    }
  }, [alerts, hasShownInitial]);

  // Separate effect for auto-hide when opened
  useEffect(() => {
    if (isOpen) {
      console.log('Popover opened, starting hide timer');
      startHideTimer();
    }

    return () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
      }
    };
  }, [isOpen]);

  // Handle click outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  if (alerts.length === 0) return null;

  // Determine the icon to show based on the highest priority alert
  const hasError = alerts.some((alert) => alert.type === AlertType.Error);
  const TriggerIcon = hasError ? IoIosCloseCircle : IoIosWarning;
  const triggerColor = hasError ? 'text-[#d7040e]' : 'text-[#cc4b03]';

  return (
    <div ref={popoverRef}>
      <Popover open={isOpen}>
        <div className="relative">
          <PopoverTrigger asChild>
            <div
              className="cursor-pointer flex items-center"
              onClick={() => {
                if (hideTimerRef.current) {
                  clearTimeout(hideTimerRef.current);
                }
                setIsOpen(!isOpen);
              }}
              onMouseEnter={() => {
                setIsOpen(true);
                if (hideTimerRef.current) {
                  clearTimeout(hideTimerRef.current);
                }
              }}
            >
              <TriggerIcon className={cn('h-5 w-5', triggerColor)} />
            </div>
          </PopoverTrigger>

          {/* Small connector area between trigger and content */}
          {isOpen && (
            <div
              className="absolute -right-2 h-6 w-8 top-full"
              onMouseEnter={() => {
                if (hideTimerRef.current) {
                  clearTimeout(hideTimerRef.current);
                }
              }}
            />
          )}

          <PopoverContent
            className="w-[275px] p-0 rounded-lg overflow-hidden"
            align="end"
            alignOffset={-100}
            sideOffset={5}
            onMouseEnter={() => {
              if (hideTimerRef.current) {
                clearTimeout(hideTimerRef.current);
              }
            }}
            onMouseLeave={() => startHideTimer(1000)}
          >
            <div className="flex flex-col">
              {alerts.map((alert, index) => (
                <div
                  key={index}
                  className={cn(
                    'flex flex-col gap-2 px-3 py-2',
                    alertStyles[alert.type],
                    index > 0 && 'border-t border-white/20'
                  )}
                >
                  <div className="flex items-center gap-2">
                    <div className="flex-shrink-0">{alertIcons[alert.type]}</div>
                    <div className="flex flex-col gap-2 flex-1">
                      <span className="text-[10px] break-words whitespace-pre-line">
                        {alert.message}
                      </span>
                      {alert.action && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            alert.action?.onClick();
                            setIsOpen(false);
                          }}
                          className="text-[10px] text-left underline hover:opacity-80 cursor-pointer outline-none"
                        >
                          {alert.action.text}
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </PopoverContent>
        </div>
      </Popover>
    </div>
  );
}
