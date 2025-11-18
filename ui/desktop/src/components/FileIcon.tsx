import React from 'react';
import { 
  Folder, 
  File, 
  Image, 
  Video, 
  Music, 
  Archive, 
  FileText, 
  Palette, 
  Code, 
  Database, 
  Settings, 
  Terminal, 
  Zap,
  BookOpen,
  Wrench
} from 'lucide-react';

interface FileIconProps {
  fileName: string;
  isDirectory: boolean;
  itemType?: 'Directory' | 'File' | 'Builtin' | 'Recipe';
  className?: string;
}

export const FileIcon: React.FC<FileIconProps> = ({
  fileName,
  isDirectory,
  itemType,
  className = 'w-4 h-4',
}) => {
  // Handle command types first
  if (itemType === 'Builtin') {
    return (
      <Zap
        className={className}
        style={{ color: '#3b82f6' }} // Blue color for builtin commands
      />
    );
  }

  if (itemType === 'Recipe') {
    return (
      <BookOpen
        className={className}
        style={{ color: '#10b981' }} // Green color for recipes
      />
    );
  }

  // Handle directories
  if (isDirectory || itemType === 'Directory') {
    return (
      <Folder
        className={className}
        style={{ color: '#f59e0b' }} // Amber color for directories
      />
    );
  }

  const ext = fileName.split('.').pop()?.toLowerCase();

  // Image files
  if (
    ['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'webp', 'bmp', 'tiff', 'tif'].includes(ext || '')
  ) {
    return (
      <Image
        className={className}
        style={{ color: '#8b5cf6' }} // Purple color for images
      />
    );
  }

  // Video files
  if (['mp4', 'mov', 'avi', 'mkv', 'webm', 'flv', 'wmv'].includes(ext || '')) {
    return (
      <Video
        className={className}
        style={{ color: '#ef4444' }} // Red color for videos
      />
    );
  }

  // Audio files
  if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'].includes(ext || '')) {
    return (
      <Music
        className={className}
        style={{ color: '#f97316' }} // Orange color for audio
      />
    );
  }

  // Archive/compressed files
  if (['zip', 'tar', 'gz', 'rar', '7z', 'bz2'].includes(ext || '')) {
    return (
      <Archive
        className={className}
        style={{ color: '#6b7280' }} // Gray color for archives
      />
    );
  }

  // PDF files
  if (ext === 'pdf') {
    return (
      <FileText
        className={className}
        style={{ color: '#dc2626' }} // Red color for PDFs
      />
    );
  }

  // Design files
  if (['ai', 'eps', 'sketch', 'fig', 'xd', 'psd'].includes(ext || '')) {
    return (
      <Palette
        className={className}
        style={{ color: '#ec4899' }} // Pink color for design files
      />
    );
  }

  // JavaScript/TypeScript files
  if (['js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs'].includes(ext || '')) {
    return (
      <Code
        className={className}
        style={{ color: '#eab308' }} // Yellow color for JS/TS
      />
    );
  }

  // Python files
  if (['py', 'pyw', 'pyc'].includes(ext || '')) {
    return (
      <Code
        className={className}
        style={{ color: '#3b82f6' }} // Blue color for Python
      />
    );
  }

  // HTML files
  if (['html', 'htm', 'xhtml'].includes(ext || '')) {
    return (
      <Code
        className={className}
        style={{ color: '#f97316' }} // Orange color for HTML
      />
    );
  }

  // CSS files
  if (['css', 'scss', 'sass', 'less', 'stylus'].includes(ext || '')) {
    return (
      <Code
        className={className}
        style={{ color: '#06b6d4' }} // Cyan color for CSS
      />
    );
  }

  // JSON/Data files
  if (['json', 'xml', 'yaml', 'yml', 'toml', 'csv'].includes(ext || '')) {
    return (
      <FileText
        className={className}
        style={{ color: '#10b981' }} // Green color for data files
      />
    );
  }

  // Markdown files
  if (['md', 'markdown', 'mdx'].includes(ext || '')) {
    return (
      <FileText
        className={className}
        style={{ color: '#6366f1' }} // Indigo color for markdown
      />
    );
  }

  // Database files
  if (['sql', 'db', 'sqlite', 'sqlite3'].includes(ext || '')) {
    return (
      <Database
        className={className}
        style={{ color: '#059669' }} // Emerald color for databases
      />
    );
  }

  // Configuration files
  if (
    [
      'env',
      'ini',
      'cfg',
      'conf',
      'config',
      'gitignore',
      'dockerignore',
      'editorconfig',
      'prettierrc',
      'eslintrc',
    ].includes(ext || '') ||
    ['dockerfile', 'makefile', 'rakefile', 'gemfile'].includes(fileName.toLowerCase())
  ) {
    return (
      <Settings
        className={className}
        style={{ color: '#6b7280' }} // Gray color for config files
      />
    );
  }

  // Text files
  if (
    ['txt', 'log', 'readme', 'license', 'changelog', 'contributing'].includes(ext || '') ||
    ['readme', 'license', 'changelog', 'contributing'].includes(fileName.toLowerCase())
  ) {
    return (
      <FileText
        className={className}
        style={{ color: '#374151' }} // Dark gray for text files
      />
    );
  }

  // Executable files
  if (['exe', 'app', 'deb', 'rpm', 'dmg', 'pkg', 'msi'].includes(ext || '')) {
    return (
      <Wrench
        className={className}
        style={{ color: '#7c3aed' }} // Purple color for executables
      />
    );
  }

  // Script files
  if (['sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'rb', 'pl', 'php'].includes(ext || '')) {
    return (
      <Terminal
        className={className}
        style={{ color: '#059669' }} // Emerald color for scripts
      />
    );
  }

  // Default file icon
  return (
    <File
      className={className}
      style={{ color: '#6b7280' }} // Gray color for unknown files
    />
  );
};
