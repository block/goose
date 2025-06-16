import React from 'react';
import { IoIosCloseCircle, IoIosWarning, IoIosInformationCircle } from 'react-icons/io';
import { cn } from '../../utils';
import { Alert, AlertType } from './types';

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
  return (
    <div className={cn('flex flex-col gap-2 px-3 py-3', alertStyles[alert.type], className)}>
      {alert.progress ? (
        <div className="flex flex-col gap-2">
          <span className="text-[11px]">{alert.message}</span>
          <div className="flex justify-between w-full">
            {[...Array(30)].map((_, i) => (
              <div
                key={i}
                className={cn(
                  'h-[2px] w-[2px] rounded-full',
                  alert.type === AlertType.Info
                    ? i < Math.round((alert.progress!.current / alert.progress!.total) * 30)
                      ? 'dark:bg-black bg-white'
                      : 'dark:bg-black/20 bg-white/20'
                    : i < Math.round((alert.progress!.current / alert.progress!.total) * 30)
                      ? 'bg-white'
                      : 'bg-white/20'
                )}
              />
            ))}
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
