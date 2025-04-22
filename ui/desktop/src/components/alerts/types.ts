export enum AlertType {
  Error = 'error',
  Warning = 'warning',
}

export interface Alert {
  type: AlertType;
  message: string;
  autoShow?: boolean;
  action?: {
    text: string;
    onClick: () => void;
  };
}
