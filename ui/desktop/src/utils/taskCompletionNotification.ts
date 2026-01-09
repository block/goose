/**
 * Shows a native OS notification when Goose completes a task.
 * Only notifies if the window is not focused (user is in another app).
 */
export function notifyTaskCompletion(): void {
  if (!document.hasFocus()) {
    window.electron.showNotification({
      title: 'Goose',
      body: 'Task completed',
    });
  }
}
