export interface SaveOptions {
  filePath?: string;
  format?: 'html' | 'markdown' | 'txt' | 'pdf';
  showDialog?: boolean;
  autoSave?: boolean;
}

export interface SaveResult {
  success: boolean;
  filePath?: string;
  error?: string;
  cancelled?: boolean;
}

export interface FileDialogOptions {
  title?: string;
  defaultPath?: string;
  filters?: Array<{
    name: string;
    extensions: string[];
  }>;
}

export interface PersistenceState {
  hasUnsavedChanges: boolean;
  lastSaved?: Date;
  filePath?: string;
  isAutoSaving: boolean;
  isSaving: boolean;
  saveError?: string;
}

export interface AutoSaveConfig {
  enabled: boolean;
  intervalMs: number;
  maxBackups: number;
  backupLocation?: string;
}

export interface DocumentMetadata {
  filePath?: string;
  fileName?: string;
  fileType?: string;
  lastModified?: Date;
  size?: number;
  encoding?: string;
}

export interface ContentSnapshot {
  content: string;
  timestamp: Date;
  checksum: string;
  metadata: DocumentMetadata;
}

export interface PersistenceActions {
  save: (options?: SaveOptions) => Promise<SaveResult>;
  saveAs: (options?: FileDialogOptions) => Promise<SaveResult>;
  load: (filePath?: string) => Promise<SaveResult>;
  autoSave: (content: string) => Promise<void>;
  createBackup: (content: string) => Promise<void>;
  restoreBackup: () => Promise<string | null>;
  hasUnsavedChanges: () => boolean;
  markAsSaved: () => void;
  markAsChanged: () => void;
}
