import React from 'react';

interface FileIconProps {
  fileName: string;
  isDirectory: boolean;
  className?: string;
}

export const FileIcon: React.FC<FileIconProps> = ({
  fileName,
  isDirectory,
  className = 'w-4 h-4',
}) => {
  if (isDirectory) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      </svg>
    );
  }

  const ext = fileName.split('.').pop()?.toLowerCase();

  // Image files
  if (
    ['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'webp', 'bmp', 'tiff', 'tif'].includes(ext || '')
  ) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <polyline points="21,15 16,10 5,21" />
      </svg>
    );
  }

  // Video files
  if (['mp4', 'mov', 'avi', 'mkv', 'webm', 'flv', 'wmv'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <polygon points="23 7 16 12 23 17 23 7" />
        <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
      </svg>
    );
  }

  // Audio files
  if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
    );
  }

  // Archive/compressed files
  if (['zip', 'tar', 'gz', 'rar', '7z', 'bz2'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M16 22h2a2 2 0 0 0 2-2V7.5L14.5 2H6a2 2 0 0 0-2 2v3" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M10 20v-1a2 2 0 1 1 4 0v1a2 2 0 1 1-4 0Z" />
        <path d="M10 7h4" />
        <path d="M10 11h4" />
      </svg>
    );
  }

  // PDF files
  if (ext === 'pdf') {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M10 12h4" />
        <path d="M10 16h2" />
        <path d="M10 8h2" />
      </svg>
    );
  }

  // Design files
  if (['ai', 'eps', 'sketch', 'fig', 'xd', 'psd'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
        <path d="M9 9h6v6h-6z" />
        <path d="M16 3.5a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v3a.5.5 0 0 1-.5.5h-3a.5.5 0 0 1-.5-.5v-3z" />
      </svg>
    );
  }

  // JavaScript/TypeScript files
  if (['js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M10 13l2 2 4-4" />
      </svg>
    );
  }

  // Python files
  if (['py', 'pyw', 'pyc'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <circle cx="10" cy="12" r="2" />
        <circle cx="14" cy="16" r="2" />
        <path d="M12 10c0-1 1-2 2-2s2 1 2 2-1 2-2 2" />
      </svg>
    );
  }

  // HTML files
  if (['html', 'htm', 'xhtml'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <polyline points="9,13 9,17 15,17 15,13" />
        <line x1="12" y1="13" x2="12" y2="17" />
      </svg>
    );
  }

  // CSS files
  if (['css', 'scss', 'sass', 'less', 'stylus'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M8 13h8" />
        <path d="M8 17h8" />
        <circle cx="12" cy="15" r="1" />
      </svg>
    );
  }

  // JSON/Data files
  if (['json', 'xml', 'yaml', 'yml', 'toml', 'csv'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M9 13v-1a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1" />
        <path d="M9 17v-1a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1" />
      </svg>
    );
  }

  // Markdown files
  if (['md', 'markdown', 'mdx'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10,9 9,9 8,9" />
      </svg>
    );
  }

  // Database files
  if (['sql', 'db', 'sqlite', 'sqlite3'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <ellipse cx="12" cy="5" rx="9" ry="3" />
        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
      </svg>
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
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1 1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    );
  }

  // Text files
  if (
    ['txt', 'log', 'readme', 'license', 'changelog', 'contributing'].includes(ext || '') ||
    ['readme', 'license', 'changelog', 'contributing'].includes(fileName.toLowerCase())
  ) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14,2 14,8 20,8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10,9 9,9 8,9" />
      </svg>
    );
  }

  // Executable files
  if (['exe', 'app', 'deb', 'rpm', 'dmg', 'pkg', 'msi'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <polygon points="14 2 18 6 18 20 6 20 6 4 14 4" />
        <polyline points="14,2 14,8 20,8" />
        <path d="M10 12l2 2 4-4" />
      </svg>
    );
  }

  // Script files
  if (['sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'rb', 'pl', 'php'].includes(ext || '')) {
    return (
      <svg
        className={className}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </svg>
    );
  }

  // Default file icon
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
    >
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14,2 14,8 20,8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
      <polyline points="10,9 9,9 8,9" />
    </svg>
  );
};
