import { app } from 'electron';
import { compareVersions } from 'compare-versions';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import log from './logger';
import { safeJsonParse } from './jsonUtils';

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

export class GitHubUpdater {
  private readonly owner = 'block';
  private readonly repo = 'goose';
  private readonly apiUrl = `https://api.github.com/repos/${this.owner}/${this.repo}/releases/latest`;

  async checkForUpdates(): Promise<UpdateCheckResult> {
    try {
      log.info('GitHubUpdater: Checking for updates via GitHub API...');
      log.info(`GitHubUpdater: API URL: ${this.apiUrl}`);
      log.info(`GitHubUpdater: Current app version: ${app.getVersion()}`);

      const response = await fetch(this.apiUrl, {
        headers: {
          Accept: 'application/vnd.github.v3+json',
          'User-Agent': `Goose-Desktop/${app.getVersion()}`,
        },
      });

      log.info(
        `GitHubUpdater: GitHub API response status: ${response.status} ${response.statusText}`
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

      const latestVersion = release.tag_name.replace(/^v/, ''); // Remove 'v' prefix if present
      const currentVersion = app.getVersion();

      log.info(
        `GitHubUpdater: Current version: ${currentVersion}, Latest version: ${latestVersion}`
      );

      // Compare versions
      const updateAvailable = compareVersions(latestVersion, currentVersion) > 0;
      log.info(`GitHubUpdater: Update available: ${updateAvailable}`);

      if (!updateAvailable) {
        return {
          updateAvailable: false,
          latestVersion,
        };
      }

      // Find the appropriate download URL based on platform
      const platform = process.platform;
      const arch = process.arch;
      let downloadUrl: string | undefined;
      let assetName: string;

      log.info(`GitHubUpdater: Looking for asset for platform: ${platform}, arch: ${arch}`);

      if (platform === 'darwin') {
        // macOS
        if (arch === 'arm64') {
          assetName = 'Goose.zip';
        } else {
          assetName = 'Goose_intel_mac.zip';
        }
      } else if (platform === 'win32') {
        // Windows - for future support
        assetName = 'Goose-win32-x64.zip';
      } else {
        // Linux - for future support
        assetName = `Goose-linux-${arch}.zip`;
      }

      log.info(`GitHubUpdater: Looking for asset named: ${assetName}`);
      log.info(`GitHubUpdater: Available assets: ${release.assets.map((a) => a.name).join(', ')}`);

      const asset = release.assets.find((a) => a.name === assetName);
      if (asset) {
        downloadUrl = asset.browser_download_url;
        log.info(`GitHubUpdater: Found matching asset: ${asset.name} (${asset.size} bytes)`);
        log.info(`GitHubUpdater: Download URL: ${downloadUrl}`);
      } else {
        log.warn(`GitHubUpdater: No matching asset found for ${assetName}`);
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
        message: error instanceof Error ? error.message : 'Unknown error',
        stack: error instanceof Error ? error.stack : 'No stack',
        name: error instanceof Error ? error.name : 'Unknown',
        code:
          error instanceof Error && 'code' in error
            ? (error as Error & { code: unknown }).code
            : undefined,
      });
      return {
        updateAvailable: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async downloadUpdate(
    downloadUrl: string,
    latestVersion: string,
    onProgress?: (percent: number) => void
  ): Promise<{ success: boolean; downloadPath?: string; extractedPath?: string; error?: string }> {
    try {
      log.info(`GitHubUpdater: Downloading update from ${downloadUrl}`);

      const response = await fetch(downloadUrl);
      if (!response.ok) {
        throw new Error(`Download failed: ${response.status} ${response.statusText}`);
      }

      // Get total size from headers
      const contentLength = response.headers.get('content-length');
      const totalSize = contentLength ? parseInt(contentLength, 10) : 0;

      if (!response.body) {
        throw new Error('Response body is null');
      }

      // Read the response stream
      const reader = response.body.getReader();
      const chunks: Uint8Array[] = [];
      let downloadedSize = 0;

      // eslint-disable-next-line no-constant-condition
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        chunks.push(value);
        downloadedSize += value.length;

        // Report progress
        if (totalSize > 0 && onProgress) {
          const percent = Math.round((downloadedSize / totalSize) * 100);
          onProgress(percent);
        }
      }

      // Combine chunks into a single buffer
      // eslint-disable-next-line no-undef
      const buffer = Buffer.concat(chunks.map((chunk) => Buffer.from(chunk)));

      // Save to Downloads directory
      const downloadsDir = path.join(os.homedir(), 'Downloads');
      const fileName = `Goose-${latestVersion}.zip`;
      const downloadPath = path.join(downloadsDir, fileName);

      await fs.writeFile(downloadPath, buffer);

      log.info(`GitHubUpdater: Update downloaded to ${downloadPath}`);

      // Return success - user will handle extraction manually
      return { success: true, downloadPath, extractedPath: downloadsDir };
    } catch (error) {
      log.error('GitHubUpdater: Error downloading update:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
}

// Create singleton instance
export const githubUpdater = new GitHubUpdater();
