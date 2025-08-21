import { AlertCircle, CheckCircle, Info, AlertTriangle } from 'lucide-react';

interface SystemAlertMessageProps {
  message: string;
  level: 'info' | 'warning' | 'error' | 'success';
  timestamp: number;
}

export default function SystemAlertMessage({ message, level, timestamp }: SystemAlertMessageProps) {
  const getAlertStyles = () => {
    switch (level) {
      case 'error':
        return {
          bgClass: 'bg-red-50 dark:bg-red-950/30',
          borderClass: 'border-red-200 dark:border-red-800',
          textClass: 'text-red-800 dark:text-red-200',
          icon: <AlertCircle className="w-4 h-4" />,
        };
      case 'warning':
        return {
          bgClass: 'bg-yellow-50 dark:bg-yellow-950/30',
          borderClass: 'border-yellow-200 dark:border-yellow-800',
          textClass: 'text-yellow-800 dark:text-yellow-200',
          icon: <AlertTriangle className="w-4 h-4" />,
        };
      case 'success':
        return {
          bgClass: 'bg-green-50 dark:bg-green-950/30',
          borderClass: 'border-green-200 dark:border-green-800',
          textClass: 'text-green-800 dark:text-green-200',
          icon: <CheckCircle className="w-4 h-4" />,
        };
      default:
        return {
          bgClass: 'bg-blue-50 dark:bg-blue-950/30',
          borderClass: 'border-blue-200 dark:border-blue-800',
          textClass: 'text-blue-800 dark:text-blue-200',
          icon: <Info className="w-4 h-4" />,
        };
    }
  };

  const { bgClass, borderClass, textClass, icon } = getAlertStyles();

  return (
    <div
      className={`flex items-start gap-3 p-3 rounded-lg border ${bgClass} ${borderClass} ${textClass} animate-fade-in`}
      data-testid="system-alert-message"
    >
      <div className="flex-shrink-0 mt-0.5">{icon}</div>
      <div className="flex-1">
        <p className="text-sm font-medium">{message}</p>
        <p className="text-xs opacity-70 mt-1">
          {new Date(timestamp).toLocaleTimeString()}
        </p>
      </div>
    </div>
  );
}
