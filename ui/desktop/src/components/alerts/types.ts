export enum AlertType {
  Error = 'error',
  Warning = 'warning',
}

export interface Alert {
  type: AlertType;
  message: string;
  action?: {
    text: string;
    onClick: () => void;
  };
}
