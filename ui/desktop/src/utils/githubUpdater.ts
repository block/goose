import { app } from 'electron';
import { compareVersions } from 'compare-versions';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import { spawn } from 'child_process';
import log from './logger';

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

      const response = await fetch(this.apiUrl, {
        headers: {
          Accept: 'application/vnd.github.v3+json',
          'User-Agent': `Goose-Desktop/${app.getVersion()}`,
        },
      });

      if (!response.ok) {
        throw new Error(`GitHub API returned ${response.status}: ${response.statusText}`);
      }

      const release: GitHubRelease = await response.json();
      const latestVersion = release.tag_name.replace(/^v/, ''); // Remove 'v' prefix if present
      const currentVersion = app.getVersion();

      log.info(
        `GitHubUpdater: Current version: ${currentVersion}, Latest version: ${latestVersion}`
      );

      // Compare versions
      const updateAvailable = compareVersions(latestVersion, currentVersion) > 0;

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

      const asset = release.assets.find((a) => a.name === assetName);
      if (asset) {
        downloadUrl = asset.browser_download_url;
      }

      return {
        updateAvailable: true,
        latestVersion,
        downloadUrl,
        releaseUrl: release.html_url,
      };
    } catch (error) {
      log.error('GitHubUpdater: Error checking for updates:', error);
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

      // Auto-unzip the downloaded file
      try {
        const tempExtractDir = path.join(downloadsDir, `temp-extract-${Date.now()}`);

        // Create temp extraction directory
        await fs.mkdir(tempExtractDir, { recursive: true });

        // Use unzip command to extract
        log.info(`GitHubUpdater: Extracting ${fileName} to temp directory`);

        const unzipProcess = spawn('unzip', ['-o', downloadPath, '-d', tempExtractDir]);

        let stderr = '';
        unzipProcess.stderr.on('data', (data) => {
          stderr += data.toString();
        });

        await new Promise<void>((resolve, reject) => {
          unzipProcess.on('close', (code) => {
            if (code === 0) {
              resolve();
            } else {
              reject(new Error(`Unzip process exited with code ${code}`));
            }
          });

          unzipProcess.on('error', (err) => {
            reject(err);
          });
        });

        if (stderr && !stderr.includes('warning')) {
          log.warn(`GitHubUpdater: Unzip stderr: ${stderr}`);
        }

        // Check if Goose.app exists in the extracted content
        const appPath = path.join(tempExtractDir, 'Goose.app');
        try {
          await fs.access(appPath);
          log.info(`GitHubUpdater: Found Goose.app at ${appPath}`);
        } catch (error) {
          log.error('GitHubUpdater: Goose.app not found in extracted content');
          throw new Error('Goose.app not found in extracted content');
        }

        // Move Goose.app to Downloads folder
        const finalAppPath = path.join(downloadsDir, 'Goose.app');

        // Remove existing Goose.app if it exists
        try {
          await fs.rm(finalAppPath, { recursive: true, force: true });
        } catch (e) {
          // File might not exist, that's fine
        }

        // Move the app to Downloads
        log.info(`GitHubUpdater: Moving Goose.app to Downloads folder`);
        await fs.rename(appPath, finalAppPath);

        // Verify the move was successful
        try {
          await fs.access(finalAppPath);
          log.info(`GitHubUpdater: Successfully moved Goose.app to Downloads`);
        } catch (error) {
          log.error('GitHubUpdater: Failed to move Goose.app');
          throw new Error('Failed to move Goose.app to Downloads');
        }

        // Clean up temp directory and zip file
        try {
          await fs.rm(tempExtractDir, { recursive: true, force: true });
          await fs.unlink(downloadPath);
          log.info(`GitHubUpdater: Cleaned up temporary files`);
        } catch (cleanupError) {
          log.warn(`GitHubUpdater: Failed to clean up temporary files: ${cleanupError}`);
        }

        return { success: true, downloadPath: finalAppPath, extractedPath: downloadsDir };
      } catch (unzipError) {
        log.error('GitHubUpdater: Error extracting update:', unzipError);
        // Still return success for download, but note the extraction error
        return {
          success: true,
          downloadPath,
          error: `Downloaded successfully but extraction failed: ${unzipError instanceof Error ? unzipError.message : 'Unknown error'}`,
        };
      }
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
