export enum AlertType {
  Error = 'error',
  Warning = 'warning',
  Info = 'info',
}

export interface Alert {
  type: AlertType;
  message: string;
  autoShow?: boolean;
  action?: {
    text: string;
    onClick: () => void;
  };
  progress?: {
    current: number;
    total: number;
  };
  showCompactButton?: boolean;
  compactButtonDisabled?: boolean;
  onCompact?: () => void;
  compactIcon?: React.ReactNode;
  autoCompactThreshold?: number; // Add this for showing the auto-compact threshold
  key?: string; // Add key for forcing re-renders
}
