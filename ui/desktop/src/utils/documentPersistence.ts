import { SaveOptions, SaveResult, FileDialogOptions, ContentSnapshot, DocumentMetadata, AutoSaveConfig } from '../types/persistence';

export class DocumentPersistence {
  private static readonly BACKUP_KEY_PREFIX = 'goose-backup-';
  private static readonly AUTOSAVE_KEY_PREFIX = 'goose-autosave-';
  private static readonly METADATA_KEY_PREFIX = 'goose-metadata-';

  // Default auto-save configuration
  private static readonly DEFAULT_AUTOSAVE_CONFIG: AutoSaveConfig = {
    enabled: true,
    intervalMs: 30000, // 30 seconds
    maxBackups: 5,
  };

  /**
   * Save content to a file with various options
   */
  static async save(content: string, options: SaveOptions = {}): Promise<SaveResult> {
    try {
      const { filePath, format = 'html', showDialog = false, autoSave = false } = options;

      if (showDialog || !filePath) {
        return await this.saveAs(content, {
          title: 'Save Document',
          defaultPath: filePath,
          filters: this.getFileFilters(format),
        });
      }

      // Convert content to appropriate format
      const formattedContent = this.formatContent(content, format, filePath);

      // Save using Electron API
      const success = await window.electron.writeFile(filePath, formattedContent);

      if (success) {
        // Update metadata
        await this.updateMetadata(filePath, content);
        
        // Clear auto-save backup if this was a manual save
        if (!autoSave) {
          this.clearAutoSave(filePath);
        }

        return {
          success: true,
          filePath,
        };
      } else {
        return {
          success: false,
          error: 'Failed to write file',
        };
      }
    } catch (error) {
      console.error('Error saving document:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Show save dialog and save content
   */
  static async saveAs(content: string, options: FileDialogOptions = {}): Promise<SaveResult> {
    try {
      // Check if the function exists
      if (!window.electron || typeof window.electron.showSaveDialog !== 'function') {
        console.error('window.electron.showSaveDialog is not available');
        return {
          success: false,
          error: 'Save dialog not available. Please restart the application.',
        };
      }

      const dialogOptions = {
        title: options.title || 'Save Document As',
        defaultPath: options.defaultPath,
        filters: options.filters || this.getDefaultFileFilters(),
      };

      console.log('Calling showSaveDialog with options:', dialogOptions);
      
      // Show save dialog
      const result = await window.electron.showSaveDialog(dialogOptions);

      if (result.cancelled || !result.filePath) {
        return {
          success: false,
          cancelled: true,
        };
      }

      // Determine format from file extension
      const format = this.getFormatFromPath(result.filePath);
      const formattedContent = this.formatContent(content, format, result.filePath);

      // Save the file
      const success = await window.electron.writeFile(result.filePath, formattedContent);

      if (success) {
        await this.updateMetadata(result.filePath, content);
        return {
          success: true,
          filePath: result.filePath,
        };
      } else {
        return {
          success: false,
          error: 'Failed to write file',
        };
      }
    } catch (error) {
      console.error('Error in saveAs:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Load content from a file
   */
  static async load(filePath?: string): Promise<SaveResult & { content?: string }> {
    try {
      let targetPath = filePath;

      if (!targetPath) {
        // Check if the function exists
        if (!window.electron || typeof window.electron.showOpenDialog !== 'function') {
          console.error('window.electron.showOpenDialog is not available');
          return {
            success: false,
            error: 'Open dialog not available. Please restart the application.',
          };
        }

        // Show open dialog
        const result = await window.electron.showOpenDialog({
          title: 'Open Document',
          filters: this.getDefaultFileFilters(),
          properties: ['openFile'],
        });

        if (result.cancelled || !result.filePaths?.[0]) {
          return {
            success: false,
            cancelled: true,
          };
        }

        targetPath = result.filePaths[0];
      }

      // Read file content
      const fileResult = await window.electron.readFile(targetPath);

      if (fileResult.found && fileResult.error === null) {
        const content = this.parseContent(fileResult.file, targetPath);
        await this.updateMetadata(targetPath, content);

        return {
          success: true,
          filePath: targetPath,
          content,
        };
      } else {
        return {
          success: false,
          error: fileResult.error || 'File not found',
        };
      }
    } catch (error) {
      console.error('Error loading document:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Auto-save content to localStorage
   */
  static async autoSave(content: string, filePath?: string): Promise<void> {
    try {
      const key = this.getAutoSaveKey(filePath);
      const snapshot: ContentSnapshot = {
        content,
        timestamp: new Date(),
        checksum: this.generateChecksum(content),
        metadata: {
          filePath,
          fileName: filePath ? this.getFileName(filePath) : undefined,
          fileType: filePath ? this.getFormatFromPath(filePath) : 'html',
          lastModified: new Date(),
          size: content.length,
        },
      };

      localStorage.setItem(key, JSON.stringify(snapshot));
      
      // Clean up old auto-saves
      this.cleanupOldAutoSaves();
    } catch (error) {
      console.error('Error auto-saving:', error);
    }
  }

  /**
   * Create a backup of the current content
   */
  static async createBackup(content: string, filePath?: string): Promise<void> {
    try {
      const key = this.getBackupKey(filePath);
      const backups = this.getBackups(filePath);
      
      const snapshot: ContentSnapshot = {
        content,
        timestamp: new Date(),
        checksum: this.generateChecksum(content),
        metadata: {
          filePath,
          fileName: filePath ? this.getFileName(filePath) : undefined,
          fileType: filePath ? this.getFormatFromPath(filePath) : 'html',
          lastModified: new Date(),
          size: content.length,
        },
      };

      backups.unshift(snapshot);
      
      // Keep only the most recent backups
      const config = this.DEFAULT_AUTOSAVE_CONFIG;
      if (backups.length > config.maxBackups) {
        backups.splice(config.maxBackups);
      }

      localStorage.setItem(key, JSON.stringify(backups));
    } catch (error) {
      console.error('Error creating backup:', error);
    }
  }

  /**
   * Restore the most recent backup
   */
  static restoreBackup(filePath?: string): string | null {
    try {
      const backups = this.getBackups(filePath);
      return backups.length > 0 ? backups[0].content : null;
    } catch (error) {
      console.error('Error restoring backup:', error);
      return null;
    }
  }

  /**
   * Get auto-saved content
   */
  static getAutoSavedContent(filePath?: string): ContentSnapshot | null {
    try {
      const key = this.getAutoSaveKey(filePath);
      const stored = localStorage.getItem(key);
      
      if (stored) {
        const snapshot = JSON.parse(stored) as ContentSnapshot;
        // Check if auto-save is recent (within 24 hours)
        const age = Date.now() - new Date(snapshot.timestamp).getTime();
        if (age < 24 * 60 * 60 * 1000) {
          return snapshot;
        }
      }
      
      return null;
    } catch (error) {
      console.error('Error getting auto-saved content:', error);
      return null;
    }
  }

  /**
   * Clear auto-save data
   */
  static clearAutoSave(filePath?: string): void {
    try {
      const key = this.getAutoSaveKey(filePath);
      localStorage.removeItem(key);
    } catch (error) {
      console.error('Error clearing auto-save:', error);
    }
  }

  /**
   * Format content based on target format and file path
   */
  private static formatContent(content: string, format: string, filePath?: string): string {
    const targetFormat = format || (filePath ? this.getFormatFromPath(filePath) : 'html');

    switch (targetFormat) {
      case 'markdown':
      case 'md':
        return this.htmlToMarkdown(content);
      case 'txt':
        return this.htmlToPlainText(content);
      case 'html':
      case 'htm':
      default:
        return content;
    }
  }

  /**
   * Parse content based on file type
   */
  private static parseContent(content: string, filePath: string): string {
    const format = this.getFormatFromPath(filePath);

    switch (format) {
      case 'markdown':
      case 'md':
        return this.markdownToHtml(content);
      case 'txt':
        return this.plainTextToHtml(content);
      case 'html':
      case 'htm':
      default:
        return content;
    }
  }

  /**
   * Convert HTML to Markdown
   */
  private static htmlToMarkdown(html: string): string {
    return html
      .replace(/<h1[^>]*>(.*?)<\/h1>/gi, '# $1\n\n')
      .replace(/<h2[^>]*>(.*?)<\/h2>/gi, '## $1\n\n')
      .replace(/<h3[^>]*>(.*?)<\/h3>/gi, '### $1\n\n')
      .replace(/<h4[^>]*>(.*?)<\/h4>/gi, '#### $1\n\n')
      .replace(/<h5[^>]*>(.*?)<\/h5>/gi, '##### $1\n\n')
      .replace(/<h6[^>]*>(.*?)<\/h6>/gi, '###### $1\n\n')
      .replace(/<strong[^>]*>(.*?)<\/strong>/gi, '**$1**')
      .replace(/<b[^>]*>(.*?)<\/b>/gi, '**$1**')
      .replace(/<em[^>]*>(.*?)<\/em>/gi, '*$1*')
      .replace(/<i[^>]*>(.*?)<\/i>/gi, '*$1*')
      .replace(/<u[^>]*>(.*?)<\/u>/gi, '_$1_')
      .replace(/<code[^>]*>(.*?)<\/code>/gi, '`$1`')
      .replace(/<pre[^>]*><code[^>]*>(.*?)<\/code><\/pre>/gis, '```\n$1\n```\n')
      .replace(/<blockquote[^>]*>(.*?)<\/blockquote>/gis, '> $1\n\n')
      .replace(/<a[^>]*href="([^"]*)"[^>]*>(.*?)<\/a>/gi, '[$2]($1)')
      .replace(/<img[^>]*src="([^"]*)"[^>]*alt="([^"]*)"[^>]*>/gi, '![$2]($1)')
      .replace(/<ul[^>]*>(.*?)<\/ul>/gis, '$1\n')
      .replace(/<ol[^>]*>(.*?)<\/ol>/gis, '$1\n')
      .replace(/<li[^>]*>(.*?)<\/li>/gi, '- $1\n')
      .replace(/<br[^>]*>/gi, '\n')
      .replace(/<p[^>]*>(.*?)<\/p>/gis, '$1\n\n')
      .replace(/<[^>]*>/g, '') // Remove remaining HTML tags
      .replace(/\n{3,}/g, '\n\n') // Normalize multiple newlines
      .trim();
  }

  /**
   * Convert HTML to plain text
   */
  private static htmlToPlainText(html: string): string {
    return html
      .replace(/<br[^>]*>/gi, '\n')
      .replace(/<p[^>]*>(.*?)<\/p>/gis, '$1\n\n')
      .replace(/<h[1-6][^>]*>(.*?)<\/h[1-6]>/gis, '$1\n\n')
      .replace(/<[^>]*>/g, '') // Remove all HTML tags
      .replace(/\n{3,}/g, '\n\n') // Normalize multiple newlines
      .trim();
  }

  /**
   * Convert Markdown to HTML
   */
  private static markdownToHtml(markdown: string): string {
    return markdown
      .replace(/^#{6}\s+(.*)$/gm, '<h6>$1</h6>')
      .replace(/^#{5}\s+(.*)$/gm, '<h5>$1</h5>')
      .replace(/^#{4}\s+(.*)$/gm, '<h4>$1</h4>')
      .replace(/^#{3}\s+(.*)$/gm, '<h3>$1</h3>')
      .replace(/^#{2}\s+(.*)$/gm, '<h2>$1</h2>')
      .replace(/^#{1}\s+(.*)$/gm, '<h1>$1</h1>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      .replace(/_(.*?)_/g, '<u>$1</u>')
      .replace(/`(.*?)`/g, '<code>$1</code>')
      .replace(/```\n([\s\S]*?)\n```/g, '<pre><code>$1</code></pre>')
      .replace(/^>\s+(.*)$/gm, '<blockquote>$1</blockquote>')
      .replace(/\[([^\]]*)\]\(([^)]*)\)/g, '<a href="$2">$1</a>')
      .replace(/!\[([^\]]*)\]\(([^)]*)\)/g, '<img src="$2" alt="$1">')
      .replace(/^-\s+(.*)$/gm, '<li>$1</li>')
      .replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
      .replace(/\n/g, '<br>')
      .replace(/<br><br>/g, '</p><p>')
      .replace(/^/, '<p>')
      .replace(/$/, '</p>');
  }

  /**
   * Convert plain text to HTML
   */
  private static plainTextToHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\n\n/g, '</p><p>')
      .replace(/\n/g, '<br>')
      .replace(/^/, '<p>')
      .replace(/$/, '</p>');
  }

  /**
   * Get file format from file path
   */
  private static getFormatFromPath(filePath: string): string {
    const extension = filePath.split('.').pop()?.toLowerCase() || '';
    switch (extension) {
      case 'md':
      case 'markdown':
        return 'markdown';
      case 'txt':
        return 'txt';
      case 'html':
      case 'htm':
        return 'html';
      default:
        return 'html';
    }
  }

  /**
   * Get file name from path
   */
  private static getFileName(filePath: string): string {
    return filePath.split('/').pop() || filePath.split('\\').pop() || filePath;
  }

  /**
   * Generate file filters for dialogs
   */
  private static getFileFilters(format?: string): Array<{ name: string; extensions: string[] }> {
    const allFilters = [
      { name: 'HTML Files', extensions: ['html', 'htm'] },
      { name: 'Markdown Files', extensions: ['md', 'markdown'] },
      { name: 'Text Files', extensions: ['txt'] },
      { name: 'All Files', extensions: ['*'] },
    ];

    if (format) {
      const formatFilter = allFilters.find(f => 
        f.extensions.includes(format) || f.name.toLowerCase().includes(format)
      );
      if (formatFilter) {
        return [formatFilter, ...allFilters.filter(f => f !== formatFilter)];
      }
    }

    return allFilters;
  }

  /**
   * Get default file filters
   */
  private static getDefaultFileFilters(): Array<{ name: string; extensions: string[] }> {
    return [
      { name: 'HTML Files', extensions: ['html', 'htm'] },
      { name: 'Markdown Files', extensions: ['md', 'markdown'] },
      { name: 'Text Files', extensions: ['txt'] },
      { name: 'All Files', extensions: ['*'] },
    ];
  }

  /**
   * Generate storage keys
   */
  private static getAutoSaveKey(filePath?: string): string {
    const identifier = filePath || 'untitled';
    return `${this.AUTOSAVE_KEY_PREFIX}${this.generateChecksum(identifier)}`;
  }

  private static getBackupKey(filePath?: string): string {
    const identifier = filePath || 'untitled';
    return `${this.BACKUP_KEY_PREFIX}${this.generateChecksum(identifier)}`;
  }

  private static getMetadataKey(filePath: string): string {
    return `${this.METADATA_KEY_PREFIX}${this.generateChecksum(filePath)}`;
  }

  /**
   * Generate simple checksum for content
   */
  private static generateChecksum(content: string): string {
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Get backups for a file
   */
  private static getBackups(filePath?: string): ContentSnapshot[] {
    try {
      const key = this.getBackupKey(filePath);
      const stored = localStorage.getItem(key);
      return stored ? JSON.parse(stored) : [];
    } catch (error) {
      console.error('Error getting backups:', error);
      return [];
    }
  }

  /**
   * Update file metadata
   */
  private static async updateMetadata(filePath: string, content: string): Promise<void> {
    try {
      const key = this.getMetadataKey(filePath);
      const metadata: DocumentMetadata = {
        filePath,
        fileName: this.getFileName(filePath),
        fileType: this.getFormatFromPath(filePath),
        lastModified: new Date(),
        size: content.length,
        encoding: 'utf-8',
      };
      
      localStorage.setItem(key, JSON.stringify(metadata));
    } catch (error) {
      console.error('Error updating metadata:', error);
    }
  }

  /**
   * Clean up old auto-saves
   */
  private static cleanupOldAutoSaves(): void {
    try {
      const keys = Object.keys(localStorage).filter(key => 
        key.startsWith(this.AUTOSAVE_KEY_PREFIX)
      );
      
      const now = Date.now();
      const maxAge = 7 * 24 * 60 * 60 * 1000; // 7 days
      
      keys.forEach(key => {
        try {
          const stored = localStorage.getItem(key);
          if (stored) {
            const snapshot = JSON.parse(stored) as ContentSnapshot;
            const age = now - new Date(snapshot.timestamp).getTime();
            
            if (age > maxAge) {
              localStorage.removeItem(key);
            }
          }
        } catch (error) {
          // If we can't parse it, remove it
          localStorage.removeItem(key);
        }
      });
    } catch (error) {
      console.error('Error cleaning up auto-saves:', error);
    }
  }
}
