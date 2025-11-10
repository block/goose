import { useState, useEffect, useRef, useCallback } from 'react';
import { FileText, ChevronRight, AlertCircle } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface SpecFile {
  name: string;
  path: string;
  lastModified: number;
}

interface SpecsViewerProps {
  projectPath: string;
}

export function SpecsViewer({ projectPath }: SpecsViewerProps) {
  const [specFiles, setSpecFiles] = useState<SpecFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<SpecFile | null>(null);
  const [fileContent, setFileContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Use a ref to always have access to the current selected file
  const selectedFileRef = useRef<SpecFile | null>(null);
  selectedFileRef.current = selectedFile;

  const loadFileContent = useCallback(async (filePath: string) => {
    try {
      setError(null);
      const content = await window.electron.readSpecFile(filePath);

      setFileContent((prevContent) => {
        if (prevContent !== content) {
          return content;
        }
        return prevContent;
      });

      setLoading(false);
    } catch (err) {
      setError('Failed to read spec file');
      setFileContent('');
      setLoading(false);
    }
  }, []);

  const loadSpecFiles = useCallback(async () => {
    try {
      setError(null);
      const files = await window.electron.listSpecFiles(projectPath);
      setSpecFiles(files);

      // Update selected file if it still exists
      setSelectedFile((prevSelected) => {
        if (prevSelected) {
          const stillExists = files.find((f) => f.path === prevSelected.path);
          if (stillExists) {
            return stillExists;
          }
          // File was deleted, select first available
          return files.length > 0 ? files[0] : null;
        } else if (files.length > 0) {
          // Auto-select first file if none selected
          return files[0];
        }
        return null;
      });

      return files;
    } catch (err) {
      setError('Failed to load spec files');
      return [];
    }
  }, [projectPath]);

  useEffect(() => {
    loadSpecFiles();
    window.electron.watchSpecDirectory(projectPath);

    return () => {
      window.electron.unwatchSpecDirectory(projectPath);
    };
  }, [projectPath, loadSpecFiles]);

  useEffect(() => {
    const handleFileChanges = (data: {
      eventType: string;
      filename: string;
      directory: string;
    }) => {
      loadSpecFiles();

      const currentSelected = selectedFileRef.current;
      if (currentSelected) {
        const currentFileName = currentSelected.path.split('/').pop() || '';

        if (currentFileName === data.filename) {
          setTimeout(() => {
            loadFileContent(currentSelected.path);
          }, 50);
        }
      }
    };

    window.electron.onSpecFilesChanged(handleFileChanges);

    return () => {
      window.electron.offSpecFilesChanged(handleFileChanges);
    };
  }, [loadSpecFiles, loadFileContent]);

  useEffect(() => {
    if (selectedFile) {
      loadFileContent(selectedFile.path);
    }
  }, [selectedFile, loadFileContent]);

  const formatFileName = (name: string) => {
    return name.replace(/\.md$/i, '').replace(/_/g, ' ');
  };

  if (specFiles.length === 0) {
    return (
      <div className="h-full flex items-center justify-center bg-background-default">
        <div className="text-center p-8">
          <FileText className="w-12 h-12 text-textMuted mx-auto mb-4" />
          <h3 className="text-lg font-medium text-textStandard mb-2">No Specs Found</h3>
          <p className="text-textMuted text-sm max-w-md">
            No markdown files found in the specs/ folder of this project. Add .md files to the
            specs/ folder to view them here.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex bg-background-default">
      {/* Sidebar with file list */}
      <div className="w-64 border-r border-borderSubtle bg-background-muted flex flex-col">
        <div className="p-3 border-b border-borderSubtle">
          <h3 className="text-sm font-medium text-textStandard">Spec Files</h3>
        </div>
        <div className="flex-1 overflow-y-auto">
          {specFiles.map((file) => (
            <button
              key={file.path}
              onClick={() => setSelectedFile(file)}
              className={`w-full px-3 py-2 flex items-center gap-2 text-left text-sm transition-colors ${
                selectedFile?.path === file.path
                  ? 'bg-background-default text-textStandard border-l-2 border-primary'
                  : 'text-textMuted hover:bg-background-default/50 hover:text-textStandard'
              }`}
            >
              <FileText size={14} className="flex-shrink-0" />
              <span className="truncate">{formatFileName(file.name)}</span>
              {selectedFile?.path === file.path && (
                <ChevronRight size={14} className="ml-auto flex-shrink-0" />
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Content area */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {error ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center p-8">
              <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
              <p className="text-textMuted">{error}</p>
            </div>
          </div>
        ) : loading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
              <p className="text-textMuted text-sm">Loading...</p>
            </div>
          </div>
        ) : selectedFile ? (
          <div className="flex-1 overflow-y-auto">
            <div className="p-6">
              <h2 className="text-xl font-semibold text-textStandard mb-4">
                {formatFileName(selectedFile.name)}
              </h2>
              <div className="prose prose-sm dark:prose-invert max-w-none">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>{fileContent}</ReactMarkdown>
              </div>
            </div>
          </div>
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <p className="text-textMuted">Select a file to view</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default SpecsViewer;
