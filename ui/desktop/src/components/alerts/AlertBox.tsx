import React from 'react';
import { IoIosCloseCircle, IoIosWarning } from 'react-icons/io';
import { cn } from '../../utils';
import { Alert, AlertType } from './types';

const alertIcons: Record<AlertType, React.ReactNode> = {
  [AlertType.Error]: <IoIosCloseCircle className="h-5 w-5" />,
  [AlertType.Warning]: <IoIosWarning className="h-5 w-5" />,
};

interface AlertBoxProps {
  alert: Alert;
  className?: string;
}

const alertStyles: Record<AlertType, string> = {
  [AlertType.Error]: 'bg-[#d7040e] text-white',
  [AlertType.Warning]: 'bg-[#cc4b03] text-white',
};

export const AlertBox = ({ alert, className }: AlertBoxProps) => {
  return (
    <div className={cn('flex items-center gap-2 px-3 py-2', alertStyles[alert.type], className)}>
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
  );
};
