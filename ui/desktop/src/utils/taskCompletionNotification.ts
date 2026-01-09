export function notifyTaskCompletion(): void {
  if (!document.hasFocus()) {
    window.electron.showNotification({
      title: 'goose',
      body: 'Task completed',
    });
  }
}
