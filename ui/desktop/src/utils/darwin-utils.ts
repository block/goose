import * as path from 'path';
import * as fs from 'fs';
import { spawn } from 'child_process';

/**
 * Utilities for macOS (Darwin) specific functionality
 * These utilities only work on macOS and should not be called on other platforms
 */
export class DarwinUtils {
  /* Verifies we are running on macOS @throws Error if not running on macOS */
  private static assertPlatform(): void {
    if (process.platform !== 'darwin') {
      throw new Error('DarwinUtils can only be used on macOS');
    }
  }

  /* Gets the path to a macOS system sound file */
  static getSoundPath(soundName: string): string {
    this.assertPlatform();
    return path.join('/System/Library/Sounds/', `${soundName}.aiff`);
  }

  /* Plays a macOS system sound using afplay */
  static async playSound(soundName: string): Promise<void> {
    this.assertPlatform();
    const filePath = this.getSoundPath(soundName);
    try {
      await fs.promises.access(filePath);
      await spawn('afplay', [filePath]);
    } catch (error) {
      console.error('[DarwinUtils] Sound error:', error);
    }
  }

  /* Opens macOS System Preferences to the Notifications pane */
  static openSystemPreferences(): void {
    this.assertPlatform();
    spawn('open', ['x-apple.systempreferences:com.apple.preference.notifications']);
  }
}
