export type NotificationType =
  | 'taskCompletion' // When Goose completes a task (including long-running tasks)
  | 'error'; // When something goes wrong or needs attention

export interface NotificationSettings {
  enabled: boolean;
  types: NotificationType[];
  onlyWhenUnfocused: boolean; // Only show notifications when app is not focused
  sounds: {
    taskCompletion: string;
    error: string;
  };
  delayBetweenNotifications: number;
}

export interface NotificationOptions {
  title: string;
  body: string;
  type: NotificationType;
  icon?: string;
  onClick?: () => void;
  sound?: string;
}
