import { Notification, BrowserWindow, IpcMainEvent } from 'electron';
import path from 'node:path';
import { NotificationData } from '../notifications/types';
import { DarwinUtils } from '../utils/darwin-utils';
import { randomUUID } from 'crypto';

export class NotificationManager {
  private activeNotifications = new Map<
    string,
    {
      notification: Notification;
      sourceWindow: BrowserWindow;
      windowId: number;
      data: NotificationData;
    }
  >();

  private getNotificationIcon(): string {
    const isDev = process.env.NODE_ENV === 'development';
    return path.join(isDev ? process.cwd() : process.resourcesPath, 'images', 'icon.png');
  }

  private handleNotificationClick(uuid: string) {
    const notificationData = this.activeNotifications.get(uuid);
    if (!notificationData) return;

    const { sourceWindow, windowId } = notificationData;
    const window = !sourceWindow.isDestroyed()
      ? sourceWindow
      : BrowserWindow.getAllWindows().find((w) => w.id === windowId) ||
        BrowserWindow.getAllWindows()[0];

    if (window) {
      if (window.isMinimized()) window.restore();
      window.show();
      window.focus();
    }
  }

  cleanupNotifications(windowId: number) {
    for (const [uuid, data] of this.activeNotifications.entries()) {
      if (data.windowId === windowId) {
        data.notification.close();
        this.activeNotifications.delete(uuid);
      }
    }
  }

  async showNotification(event: IpcMainEvent, data: NotificationData) {
    const sourceWindow = BrowserWindow.fromWebContents(event.sender);
    if (!sourceWindow) throw new Error('Could not find source window');

    const uuid = randomUUID();

    const notification = new Notification({
      title: data.title,
      body: data.body,
      silent: data.sound && data.sound !== 'default' ? true : false,
      icon: this.getNotificationIcon(),
      urgency: data.type === 'error' ? 'critical' : 'normal',
      subtitle: 'Goose Desktop',
      hasReply: false,
      timeoutType: 'default',
      actions: (data.actions || []).map((a) => ({ type: 'button', text: a.text })),
    });

    // Play custom sound on macOS if requested
    if (process.platform === 'darwin' && data.sound && data.sound !== 'default') {
      await DarwinUtils.playSound(data.sound);
    }

    this.activeNotifications.set(uuid, {
      notification,
      sourceWindow,
      windowId: sourceWindow.id,
      data, // store data if needed
    });

    notification.on('click', () => {
      this.handleNotificationClick(uuid);
      if (data.onClick) data.onClick();
    });

    notification.on('close', () => {
      this.activeNotifications.delete(uuid);
    });

    notification.show();

    // Optionally return the UUID to the renderer if you need to reference it later
    return uuid;
  }

  closeNotification(uuid: string) {
    const notificationData = this.activeNotifications.get(uuid);
    if (notificationData) {
      notificationData.notification.close();
      this.activeNotifications.delete(uuid);
    }
  }
}
