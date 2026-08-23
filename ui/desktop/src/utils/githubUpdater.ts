import { app } from 'electron';
import { compareVersions } from 'compare-versions';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import log from './logger';
import { safeJsonParse, errorMessage } from './conversionUtils';

interface GitHubRelease {
  tag_name: string;
  name: string;
  published_at: string;
  html_url: string;
  assets: Array<{
    name: string;
    browser_download_url: string;
    size: number;
  }>;
}

interface UpdateCheckResult {
  updateAvailable: boolean;
  latestVersion?: string;
  downloadUrl?: string;
  releaseUrl?: string;
  error?: string;
}

export function resolveUpdateAssetName(
  platform: typeof process.platform,
  arch: string,
  bundleName = process.env.GOOSE_BUNDLE_NAME || 'Avocado Work'
): string {
  if (platform === 'darwin') {
    return arch === 'arm64' ? `${bundleName}.zip` : `${bundleName}_intel_mac.zip`;
  }
  if (platform === 'win32') {
    return `${bundleName}-Setup-x64.exe`;
  }
  return `${bundleName}-linux-${arch}.zip`;
}

/**
 * Release assets are versioned (e.g. `Avocado Work-1.45.0.zip`) and GitHub
 * replaces spaces with periods in asset names, so match by shape rather than an
 * exact filename. Mirrors `assetMatchers` in scripts/release-assets.js.
 */
export function updateAssetPattern(
  platform: typeof process.platform,
  arch: string,
  bundleName = process.env.GOOSE_BUNDLE_NAME || 'Avocado Work'
): RegExp {
  const escaped = bundleName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/ /g, '[ .]');
  // The version suffix is "-<semver>"; GitHub also sanitizes spaces to periods.
  const ver = '(?:[-_. ]?[0-9][0-9A-Za-z.+-]*)?';
  if (platform === 'darwin') {
    return arch === 'arm64'
      ? new RegExp(`^${escaped}${ver}\\.zip$`, 'i')
      : new RegExp(`^${escaped}${ver}_intel_mac\\.zip$`, 'i');
  }
  if (platform === 'win32') {
    return new RegExp(`^${escaped}-Setup${ver}-x64\\.exe$`, 'i');
  }
  return new RegExp(`^${escaped}-linux-${arch}${ver}\\.zip$`, 'i');
}

const SEMVER_PREFIX = /^\d+\.\d+\.\d+/;

/** Pull a semver out of versioned asset names when the tag isn't itself semver. */
export function extractVersionFromAssets(assets: Array<{ name: string }>): string | undefined {
  for (const asset of assets) {
    const match = asset.name.match(/[-.](\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)(?:_intel_mac)?\.(?:dmg|zip|exe)$/i);
    if (match) {
      return match[1];
    }
  }
  return undefined;
}

export class GitHubUpdater {
  private readonly owner = process.env.GITHUB_OWNER || 'Avocado-Technology';
  private readonly repo = process.env.GITHUB_REPO || 'avcd-agent';
  private readonly bundleName = process.env.GOOSE_BUNDLE_NAME || 'Avocado Work';
  private readonly apiUrl = `https://api.github.com/repos/${this.owner}/${this.repo}/releases/latest`;

  async checkForUpdates(): Promise<UpdateCheckResult> {
    const startTime = Date.now();
    try {
      log.info('=== GitHubUpdater: STARTING UPDATE CHECK ===');
      log.info(`GitHubUpdater: API URL: ${this.apiUrl}`);
      log.info(`GitHubUpdater: Current app version: ${app.getVersion()}`);
      log.info(`GitHubUpdater: Timestamp: ${new Date().toISOString()}`);

      log.info('GitHubUpdater: Initiating fetch request...');
      const controller = new AbortController();
      const timeoutId = setTimeout(() => {
        log.error('GitHubUpdater: Fetch request timed out after 30 seconds');
        controller.abort();
      }, 30000);

      const response = await fetch(this.apiUrl, {
        headers: {
          Accept: 'application/vnd.github.v3+json',
          'User-Agent': `AVCD-Agent-Desktop/${app.getVersion()}`,
        },
        signal: controller.signal,
      });

      clearTimeout(timeoutId);
      const fetchDuration = Date.now() - startTime;
      log.info(
        `GitHubUpdater: GitHub API response status: ${response.status} ${response.statusText} (took ${fetchDuration}ms)`
      );

      if (!response.ok) {
        const errorText = await response.text();
        log.error(`GitHubUpdater: GitHub API error response: ${errorText}`);
        throw new Error(`GitHub API returned ${response.status}: ${response.statusText}`);
      }

      const release: GitHubRelease = await safeJsonParse<GitHubRelease>(
        response,
        'Failed to get GitHub release information'
      );
      log.info(`GitHubUpdater: Found release: ${release.tag_name} (${release.name})`);
      log.info(`GitHubUpdater: Release published at: ${release.published_at}`);
      log.info(`GitHubUpdater: Release assets count: ${release.assets.length}`);

      let latestVersion = release.tag_name.replace(/^v/, ''); // Remove 'v' prefix if present
      // The `stable` pointer tag isn't a semver; recover the real version from
      // the versioned asset names so version comparison still works.
      if (!SEMVER_PREFIX.test(latestVersion)) {
        const fromAssets = extractVersionFromAssets(release.assets);
        if (fromAssets) {
          log.info(
            `GitHubUpdater: tag "${release.tag_name}" is not semver; using ${fromAssets} from assets`
          );
          latestVersion = fromAssets;
        }
      }
      const currentVersion = app.getVersion();

      log.info(
        `GitHubUpdater: Current version: ${currentVersion}, Latest version: ${latestVersion}`
      );

      // Compare versions (guard against a non-semver tag we couldn't recover).
      let updateAvailable = false;
      try {
        updateAvailable = compareVersions(latestVersion, currentVersion) > 0;
      } catch (compareError) {
        log.warn(
          `GitHubUpdater: Could not compare versions ("${latestVersion}" vs "${currentVersion}"): ${errorMessage(compareError, 'unknown')}`
        );
      }
      log.info(`GitHubUpdater: Update available: ${updateAvailable}`);

      if (!updateAvailable) {
        return {
          updateAvailable: false,
          latestVersion,
        };
      }

      const platform = process.platform;
      const arch = process.arch;
      let downloadUrl: string | undefined;
      const assetPattern = updateAssetPattern(platform, arch, this.bundleName);

      log.info(`GitHubUpdater: Looking for asset for platform: ${platform}, arch: ${arch}`);
      log.info(`GitHubUpdater: Matching asset pattern: ${assetPattern}`);
      log.info(`GitHubUpdater: Available assets: ${release.assets.map((a) => a.name).join(', ')}`);

      const asset = release.assets.find((a) => assetPattern.test(a.name));
      if (asset && /^Goose(\.zip|_intel_mac\.zip|-win32-|-Setup)/i.test(asset.name)) {
        throw new Error(`Refusing Goose-named update asset: ${asset.name}`);
      }
      if (asset) {
        downloadUrl = asset.browser_download_url;
        log.info(`GitHubUpdater: Found matching asset: ${asset.name} (${asset.size} bytes)`);
        log.info(`GitHubUpdater: Download URL: ${downloadUrl}`);
      } else {
        log.warn(`GitHubUpdater: No matching asset found for pattern ${assetPattern}`);
      }

      if (!downloadUrl) {
        throw new Error(
          `Update Available but no download URL found for platform: ${platform}, arch: ${arch}`
        );
      }

      return {
        updateAvailable: true,
        latestVersion,
        downloadUrl,
        releaseUrl: release.html_url,
      };
    } catch (error) {
      log.error('GitHubUpdater: Error checking for updates:', error);
      log.error('GitHubUpdater: Error details:', {
        message: errorMessage(error, 'Unknown error'),
        stack: error instanceof Error ? error.stack : 'No stack',
        name: error instanceof Error ? error.name : 'Unknown',
        code:
          error instanceof Error && 'code' in error
            ? (error as Error & { code: unknown }).code
            : undefined,
      });
      return {
        updateAvailable: false,
        error: errorMessage(error, 'Unknown error'),
      };
    }
  }

  async downloadUpdate(
    downloadUrl: string,
    latestVersion: string,
    onProgress?: (percent: number) => void
  ): Promise<{ success: boolean; downloadPath?: string; extractedPath?: string; error?: string }> {
    const downloadStartTime = Date.now();
    try {
      log.info('=== GitHubUpdater: STARTING DOWNLOAD ===');
      log.info(`GitHubUpdater: Download URL: ${downloadUrl}`);
      log.info(`GitHubUpdater: Version: ${latestVersion}`);
      log.info(`GitHubUpdater: Timestamp: ${new Date().toISOString()}`);

      log.info('GitHubUpdater: Initiating download fetch request...');
      const response = await fetch(downloadUrl);
      const fetchDuration = Date.now() - downloadStartTime;
      log.info(
        `GitHubUpdater: Download response received in ${fetchDuration}ms - Status: ${response.status} ${response.statusText}`
      );

      if (!response.ok) {
        throw new Error(`Download failed: ${response.status} ${response.statusText}`);
      }

      // Get total size from headers
      const contentLength = response.headers.get('content-length');
      const totalSize = contentLength ? parseInt(contentLength, 10) : 0;
      log.info(
        `GitHubUpdater: Content-Length: ${totalSize} bytes (${(totalSize / 1024 / 1024).toFixed(2)} MB)`
      );

      if (!response.body) {
        throw new Error('Response body is null');
      }
      let lastReportedPercent = -1; // Track last reported percentage to throttle updates
      let lastLoggedPercent = -1; // Track for logging at 10% intervals

      // Read the response stream
      log.info('GitHubUpdater: Starting to read response stream...');
      const reader = response.body.getReader();
      const chunks: Uint8Array[] = [];
      let downloadedSize = 0;
      let lastProgressTime = Date.now();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        chunks.push(value);
        downloadedSize += value.length;

        // Report progress - only when percentage changes by at least 1%
        if (totalSize > 0 && onProgress) {
          const percent = Math.round((downloadedSize / totalSize) * 100);

          // Only report if percent changed (throttles from hundreds/sec to ~100 total)
          if (percent !== lastReportedPercent) {
            onProgress(percent);
            lastReportedPercent = percent;

            // Log at 10% intervals for debugging
            if (percent % 10 === 0 && percent !== lastLoggedPercent) {
              const elapsed = Date.now() - downloadStartTime;
              const speed = downloadedSize / (elapsed / 1000) / 1024; // KB/s
              log.info(
                `GitHubUpdater: Download progress ${percent}% (${(downloadedSize / 1024 / 1024).toFixed(2)}/${(totalSize / 1024 / 1024).toFixed(2)} MB) @ ${speed.toFixed(0)} KB/s`
              );
              lastLoggedPercent = percent;
            }
          }
        }

        // Warn if no progress for 30 seconds
        const now = Date.now();
        if (now - lastProgressTime > 30000) {
          log.warn(
            `GitHubUpdater: Download appears slow - no significant progress in 30 seconds (${downloadedSize}/${totalSize} bytes)`
          );
          lastProgressTime = now;
        } else if (value.length > 0) {
          lastProgressTime = now;
        }
      }

      const downloadDuration = Date.now() - downloadStartTime;
      const avgSpeed = downloadedSize / (downloadDuration / 1000) / 1024;
      log.info(
        `GitHubUpdater: Download stream complete - ${downloadedSize} bytes in ${downloadDuration}ms (avg ${avgSpeed.toFixed(0)} KB/s)`
      );

      // Combine chunks into a single buffer
      log.info('GitHubUpdater: Combining chunks into buffer...');
      const buffer = Buffer.concat(chunks.map((chunk) => Buffer.from(chunk)));
      log.info(`GitHubUpdater: Buffer created - ${buffer.length} bytes`);

      const downloadsDir = path.join(os.homedir(), 'Downloads');
      const remoteName = path.basename(new URL(downloadUrl).pathname);
      const extension = path.extname(remoteName) || '.zip';
      const fileName = `${this.bundleName}-${latestVersion}${extension}`;
      const downloadPath = path.join(downloadsDir, fileName);

      log.info(`GitHubUpdater: Writing file to ${downloadPath}...`);
      await fs.writeFile(downloadPath, buffer);

      const totalDuration = Date.now() - downloadStartTime;
      log.info(`=== GitHubUpdater: DOWNLOAD COMPLETE in ${totalDuration}ms ===`);
      log.info(`GitHubUpdater: File saved to ${downloadPath}`);

      // Return success - user will handle extraction manually
      return { success: true, downloadPath, extractedPath: downloadsDir };
    } catch (error) {
      const duration = Date.now() - downloadStartTime;
      log.error(`=== GitHubUpdater: DOWNLOAD FAILED after ${duration}ms ===`);
      log.error('GitHubUpdater: Error downloading update:', error);
      log.error('GitHubUpdater: Download error details:', {
        message: errorMessage(error, 'Unknown error'),
        stack: error instanceof Error ? error.stack : 'No stack',
        name: error instanceof Error ? error.name : 'Unknown',
      });
      return {
        success: false,
        error: errorMessage(error, 'Unknown error'),
      };
    }
  }
}

// Create singleton instance
export const githubUpdater = new GitHubUpdater();
