import { promisify } from 'util';
import { execFile } from 'child_process';
import { join } from 'path';
import * as fs from 'fs';
import { expect, Page } from '@playwright/test';
import { debugLog } from './debug-log';

const execFileAsync = promisify(execFile);

export const isVideoRecording = () => process.env.PW_ELECTRON_VIDEO === '1';

export async function enableCursorHighlight(page: Page): Promise<void> {
  try {
    await page.evaluate(() => {
      const markerAttr = 'data-test-cursor-highlight';
      if (document.querySelector(`[${markerAttr}]`)) {
        return;
      }

      const style = document.createElement('style');
      style.setAttribute(markerAttr, 'style');
      style.textContent = `
        .test-cursor-overlay {
          position: fixed;
          inset: 0;
          pointer-events: none;
          z-index: 2147483647;
          overflow: hidden;
        }
        .test-cursor-highlight {
          position: absolute;
          width: 22px;
          height: 22px;
          border-radius: 9999px;
          border: 2px solid rgba(255, 255, 255, 0.95);
          background: rgba(239, 68, 68, 0.45);
          box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.2), 0 4px 14px rgba(0, 0, 0, 0.25);
          transform: none;
          transition: width 100ms ease, height 100ms ease, background 100ms ease;
          mix-blend-mode: normal;
          opacity: 0;
        }
        .test-cursor-highlight.clicking {
          width: 28px;
          height: 28px;
          background: rgba(239, 68, 68, 0.65);
        }
        .test-cursor-click-ring {
          position: absolute;
          width: 12px;
          height: 12px;
          border-radius: 9999px;
          border: 2px solid rgba(239, 68, 68, 0.8);
          transform: none;
          animation: test-cursor-ring 420ms ease-out forwards;
        }
        @keyframes test-cursor-ring {
          0% { opacity: 0.95; width: 12px; height: 12px; }
          100% { opacity: 0; width: 54px; height: 54px; }
        }
      `;
      document.head.appendChild(style);

      const overlay = document.createElement('div');
      overlay.className = 'test-cursor-overlay';
      overlay.setAttribute(markerAttr, 'overlay');
      document.body.appendChild(overlay);

      const cursor = document.createElement('div');
      cursor.className = 'test-cursor-highlight';
      cursor.setAttribute(markerAttr, 'cursor');
      overlay.appendChild(cursor);

      const setElementCenter = (element: HTMLElement, x: number, y: number) => {
        const width = element.offsetWidth || Number.parseInt(getComputedStyle(element).width, 10) || 0;
        const height = element.offsetHeight || Number.parseInt(getComputedStyle(element).height, 10) || 0;
        element.style.left = `${x - width / 2}px`;
        element.style.top = `${y - height / 2}px`;
      };

      const moveCursor = (x: number, y: number) => {
        setElementCenter(cursor, x, y);
        cursor.style.opacity = '1';
      };

      const spawnClickRing = (x: number, y: number) => {
        const ring = document.createElement('div');
        ring.className = 'test-cursor-click-ring';
        setElementCenter(ring, x, y);
        ring.setAttribute(markerAttr, 'ring');
        overlay.appendChild(ring);
        window.setTimeout(() => ring.remove(), 500);
      };

      document.addEventListener(
        'mousemove',
        (event) => {
          moveCursor(event.clientX, event.clientY);
        },
        { passive: true }
      );

      document.addEventListener(
        'mousedown',
        (event) => {
          moveCursor(event.clientX, event.clientY);
          cursor.classList.add('clicking');
          spawnClickRing(event.clientX, event.clientY);
        },
        { passive: true }
      );

      document.addEventListener(
        'mouseup',
        () => {
          cursor.classList.remove('clicking');
        },
        { passive: true }
      );
    });
  } catch (error) {
    debugLog(`Failed to enable cursor highlight: ${String(error)}`);
  }
}

export async function trimVideosInDirectory(videoDir: string, trimStartMs: number): Promise<void> {
  const requestedTrimSeconds = Math.max(0, trimStartMs) / 1000;
  if (requestedTrimSeconds <= 0 || !fs.existsSync(videoDir)) {
    return;
  }

  const files = fs.readdirSync(videoDir).filter((name) => name.endsWith('.webm'));
  if (files.length === 0) {
    return;
  }

  for (const fileName of files) {
    const sourcePath = join(videoDir, fileName);
    const trimmedPath = join(videoDir, `${fileName}.trimmed.webm`);
    try {
      const durationSeconds = await getVideoDurationSeconds(sourcePath);
      const maxTrimSeconds = Math.max(0, durationSeconds - 0.25);
      const trimSeconds = Math.min(requestedTrimSeconds, maxTrimSeconds);
      if (trimSeconds <= 0) {
        continue;
      }

      await execFileAsync('ffmpeg', [
        '-y',
        '-ss',
        trimSeconds.toFixed(3),
        '-i',
        sourcePath,
        '-c:v',
        'libvpx-vp9',
        '-b:v',
        '0',
        '-crf',
        '32',
        '-an',
        trimmedPath
      ]);
      fs.renameSync(trimmedPath, sourcePath);
    } catch (error) {
      debugLog(`Failed to trim video ${sourcePath}: ${String(error)}`);
      if (fs.existsSync(trimmedPath)) {
        fs.rmSync(trimmedPath, { force: true });
      }
    }
  }
}

async function getVideoDurationSeconds(videoPath: string): Promise<number> {
  try {
    const { stdout } = await execFileAsync('ffprobe', [
      '-v',
      'error',
      '-show_entries',
      'format=duration',
      '-of',
      'default=noprint_wrappers=1:nokey=1',
      videoPath
    ]);
    const duration = Number(stdout.trim());
    return Number.isFinite(duration) && duration > 0 ? duration : 0;
  } catch (error) {
    debugLog(`Failed to probe video duration ${videoPath}: ${String(error)}`);
    return 0;
  }
}

export async function waitForLoadingDone(page: Page, timeout: number): Promise<void> {
  await expect(page.getByTestId('loading-indicator')).toHaveCount(0, { timeout });
  if (isVideoRecording()) {
    await page.waitForTimeout(1000);
  }
}
