import React, { useRef, useEffect, useCallback } from 'react';
import { IoIosCloseCircle, IoIosWarning } from 'react-icons/io';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { cn } from '../../utils';
import { Alert, AlertType } from '../alerts';
import { AlertBox } from '../alerts';

const { clearTimeout } = window;

interface AlertPopoverProps {
  alerts: Alert[];
}

export default function BottomMenuAlertPopover({ alerts }: AlertPopoverProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [hasShownInitial, setHasShownInitial] = React.useState(false);
  const [isHovered, setIsHovered] = React.useState(false);
  const [wasAutoShown, setWasAutoShown] = React.useState(false);
  const previousAlertsRef = useRef<Alert[]>([]);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const popoverRef = useRef<HTMLDivElement>(null);

  // Function to start the hide timer
  const startHideTimer = useCallback((duration = 3000) => {
    // Clear any existing timer
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    // Start new timer
    hideTimerRef.current = setTimeout(() => {
      setIsOpen(false);
      setWasAutoShown(false);
    }, duration);
  }, []);

  // Handle initial show and new alerts
  useEffect(() => {
    if (alerts.length === 0) return;

    // Compare current and previous alerts for any changes
    const hasChanges = alerts.some((alert, index) => {
      const prevAlert = previousAlertsRef.current[index];
      return !prevAlert || prevAlert.type !== alert.type || prevAlert.message !== alert.message;
    });

    previousAlertsRef.current = alerts;

    // Auto show the popover if there are new alerts
    if (!hasShownInitial || hasChanges) {
      setIsOpen(true);
      setHasShownInitial(true);
      setWasAutoShown(true);
      // Start 3 second timer for auto-show
      startHideTimer(3000);
    }
  }, [alerts, hasShownInitial, startHideTimer]);

  // Handle auto-hide based on hover state changes
  useEffect(() => {
    if (!isHovered && isOpen && !wasAutoShown) {
      // Only start 1 second timer for manual interactions
      startHideTimer(1000);
    }
  }, [isHovered, isOpen, startHideTimer, wasAutoShown]);

  // Handle click outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        setIsOpen(false);
        setWasAutoShown(false);
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
                setWasAutoShown(false);
                setIsOpen(!isOpen);
              }}
              onMouseEnter={() => {
                setIsOpen(true);
                setIsHovered(true);
                setWasAutoShown(false);
                if (hideTimerRef.current) {
                  clearTimeout(hideTimerRef.current);
                }
              }}
              onMouseLeave={() => {
                setIsHovered(false);
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
                setIsHovered(true);
                if (hideTimerRef.current) {
                  clearTimeout(hideTimerRef.current);
                }
              }}
              onMouseLeave={() => {
                setIsHovered(false);
              }}
            />
          )}

          <PopoverContent
            className="w-[275px] p-0 rounded-lg overflow-hidden"
            align="end"
            alignOffset={-100}
            sideOffset={5}
            onMouseEnter={() => {
              setIsHovered(true);
              if (hideTimerRef.current) {
                clearTimeout(hideTimerRef.current);
              }
            }}
            onMouseLeave={() => {
              setIsHovered(false);
            }}
          >
            <div className="flex flex-col">
              {alerts.map((alert, index) => (
                <div key={index} className={cn(index > 0 && 'border-t border-white/20')}>
                  <AlertBox alert={alert} />
                </div>
              ))}
            </div>
          </PopoverContent>
        </div>
      </Popover>
    </div>
  );
}
