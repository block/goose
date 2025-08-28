import React, { useState, useEffect } from 'react';
import { FileText, AlertCircle } from 'lucide-react';

interface FileViewerProps {
  filePath: string;
}

interface FileReadResult {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

export const FileViewer: React.FC<FileViewerProps> = ({ filePath }) => {
  const [content, setContent] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadFile = async () => {
      setIsLoading(true);
      setError(null);
      
      try {
        console.log('Loading file:', filePath);
        
        // Use Electron's readFile API - returns an object with file content
        const result = await window.electron.readFile(filePath) as FileReadResult;
        
        console.log('File read result:', result);
        
        if (result.found && result.error === null) {
          console.log('File loaded successfully, length:', result.file.length);
          setContent(result.file);
        } else {
          const errorMessage = result.error || 'File not found';
          console.error('Error reading file:', errorMessage);
          setError(errorMessage);
        }
      } catch (err) {
        console.error('Error reading file:', err);
        setError(err instanceof Error ? err.message : 'Failed to read file');
      } finally {
        setIsLoading(false);
      }
    };

    if (filePath) {
      loadFile();
    }
  }, [filePath]);

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center bg-background-default">
        <div className="flex items-center space-x-2 text-text-muted">
          <FileText className="w-5 h-5 animate-pulse" />
          <span>Loading file...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center bg-background-default">
        <div className="flex flex-col items-center space-y-2 text-text-muted">
          <AlertCircle className="w-8 h-8 text-red-500" />
          <span className="text-sm">Error loading file</span>
          <span className="text-xs text-text-subtle">{error}</span>
        </div>
      </div>
    );
  }

  // Get file extension for syntax highlighting hint
  const fileExtension = filePath.split('.').pop()?.toLowerCase() || '';
  const getLanguage = (ext: string) => {
    const languageMap: { [key: string]: string } = {
      'js': 'javascript',
      'jsx': 'javascript',
      'ts': 'typescript',
      'tsx': 'typescript',
      'py': 'python',
      'json': 'json',
      'css': 'css',
      'html': 'html',
      'xml': 'xml',
      'md': 'markdown',
      'yaml': 'yaml',
      'yml': 'yaml',
      'toml': 'toml',
      'rs': 'rust',
      'go': 'go',
      'java': 'java',
      'cpp': 'cpp',
      'c': 'c',
      'cs': 'csharp',
      'php': 'php',
      'rb': 'ruby',
      'swift': 'swift'
    };
    return languageMap[ext] || 'text';
  };

  const language = getLanguage(fileExtension);

  return (
    <div className="h-full flex flex-col bg-background-default">
      {/* File header */}
      <div className="flex-shrink-0 px-4 py-2 border-b border-border-subtle bg-background-muted">
        <div className="flex items-center space-x-2">
          <FileText className="w-4 h-4 text-text-muted" />
          <span className="text-sm font-mono text-text-standard truncate">
            {filePath.split('/').pop() || filePath}
          </span>
          {language !== 'text' && (
            <span className="text-xs text-text-subtle bg-background-default px-2 py-1 rounded">
              {language}
            </span>
          )}
        </div>
        <div className="text-xs text-text-subtle mt-1 font-mono truncate">
          {filePath}
        </div>
      </div>

      {/* File content */}
      <div className="flex-1 overflow-auto">
        <pre className="p-4 text-sm font-mono text-text-standard whitespace-pre-wrap break-words">
          <code className={`language-${language}`}>{content}</code>
        </pre>
      </div>
    </div>
  );
};
