import React, { useState } from 'react';
import { Button } from './ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Download, Copy, Check, X, Edit3, Save, Eye, EyeOff, Folder, File } from 'lucide-react';

// Note: JSZip would need to be installed as a dependency
// For now, we'll use a simple text-based approach
declare global {
  interface Window {
    JSZip?: unknown;
  }
}

interface NextJSFile {
  name: string;
  content: string;
  language: string;
  path: string;
}

interface NextJSProjectArtifactProps {
  files: NextJSFile[];
  title?: string;
  description?: string;
  onSave?: (files: NextJSFile[], title: string) => void;
  onDelete?: () => void;
}

export const NextJSProjectArtifact: React.FC<NextJSProjectArtifactProps> = ({
  files,
  title = 'Next.js Project',
  description,
  onSave,
  onDelete,
}) => {
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editedFiles, setEditedFiles] = useState<NextJSFile[]>(files);
  const [editedTitle, setEditedTitle] = useState(title);
  const [selectedFile, setSelectedFile] = useState<NextJSFile | null>(files[0] || null);
  const [copied, setCopied] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    if (onSave) {
      onSave(editedFiles, editedTitle);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    }
    setIsEditing(false);
  };

  const handleCancel = () => {
    setEditedFiles(files);
    setEditedTitle(title);
    setIsEditing(false);
  };

  const handleCopy = async () => {
    if (!selectedFile) return;

    try {
      await navigator.clipboard.writeText(selectedFile.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  const handleDownload = () => {
    // For now, create a simple text file with all project files
    // In a real implementation, you would use JSZip library
    let content = `# ${editedTitle}\n\n`;
    content += `Generated on: ${new Date().toLocaleDateString()}\n\n`;

    editedFiles.forEach((file) => {
      content += `## ${file.path}\n`;
      content += `\`\`\`${file.language}\n${file.content}\n\`\`\`\n\n`;
    });

    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${editedTitle.replace(/\s+/g, '-').toLowerCase()}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const generatePreviewHTML = () => {
    // Find the main page file (usually pages/index.js or app/page.js)
    const mainFile = editedFiles.find(
      (file) =>
        file.path.includes('pages/index') ||
        file.path.includes('app/page') ||
        file.path.includes('index.html')
    );

    if (!mainFile) {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Next.js Project Preview</title>
          <script src="https://cdn.tailwindcss.com"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .file-list { background: #f8f9fa; padding: 15px; border-radius: 4px; margin-bottom: 20px; }
            .file-item { padding: 5px 0; font-family: 'Courier New', monospace; }
            .file-item:before { content: "📄 "; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>Next.js Project: ${editedTitle}</h1>
            <p>This is a Next.js project with the following files:</p>
            <div class="file-list">
              ${editedFiles.map((file) => `<div class="file-item">${file.path}</div>`).join('')}
            </div>
            <p><strong>Note:</strong> To run this project, you need to:</p>
            <ol>
              <li>Extract the downloaded ZIP file</li>
              <li>Run <code>npm install</code> or <code>yarn install</code></li>
              <li>Run <code>npm run dev</code> or <code>yarn dev</code></li>
            </ol>
          </div>
        </body>
        </html>
      `;
    }

    // For React/Next.js files, we need a more sophisticated setup
    if (
      mainFile.language === 'jsx' ||
      mainFile.language === 'tsx' ||
      mainFile.language === 'javascript'
    ) {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Next.js Project Preview</title>
          <script src="https://unpkg.com/react@18/umd/react.development.js"></script>
          <script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
          <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
          <script src="https://cdn.tailwindcss.com"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .file-list { background: #f8f9fa; padding: 15px; border-radius: 4px; margin-bottom: 20px; }
            .file-item { padding: 5px 0; font-family: 'Courier New', monospace; }
            .file-item:before { content: "📄 "; }
            .error { color: red; background: #ffe6e6; padding: 10px; border-radius: 4px; margin: 10px 0; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>Next.js Project: ${editedTitle}</h1>
            <div class="file-list">
              ${editedFiles.map((file) => `<div class="file-item">${file.path}</div>`).join('')}
            </div>
            <div id="root"></div>
            <div id="error-container"></div>
          </div>
          
          <script type="text/babel">
            try {
              ${mainFile.content}
              
              // Try to render the component
              const rootElement = document.getElementById('root');
              const errorContainer = document.getElementById('error-container');
              
              // Check if App component exists
              if (typeof App !== 'undefined') {
                ReactDOM.render(React.createElement(App), rootElement);
              } else if (typeof Page !== 'undefined') {
                ReactDOM.render(React.createElement(Page), rootElement);
              } else {
                // If no main component, try to render the first exported component
                const componentNames = Object.keys(window).filter(key => 
                  typeof window[key] === 'function' && 
                  key[0] === key[0].toUpperCase()
                );
                
                if (componentNames.length > 0) {
                  ReactDOM.render(React.createElement(window[componentNames[0]]), rootElement);
                } else {
                  errorContainer.innerHTML = '<div class="error">Keine React-Komponente gefunden. Dies ist ein Next.js Projekt - bitte lade es herunter und führe es lokal aus.</div>';
                }
              }
            } catch (error) {
              document.getElementById('error-container').innerHTML = '<div class="error">Fehler beim Rendern der Komponente: ' + error.message + '</div>';
              console.error('React rendering error:', error);
            }
          </script>
        </body>
        </html>
      `;
    }

    return mainFile.content;
  };

  return (
    <div className="border border-border-default rounded-lg bg-background-default overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border-default">
        <div className="flex items-center gap-3">
          <Folder className="h-5 w-5 text-blue-500" />
          <div>
            {isEditing ? (
              <input
                type="text"
                value={editedTitle}
                onChange={(e) => setEditedTitle(e.target.value)}
                className="text-lg font-semibold bg-transparent border-b border-border-default focus:outline-none focus:border-blue-500"
              />
            ) : (
              <h3 className="text-lg font-semibold text-text-default">{editedTitle}</h3>
            )}
            {description && <p className="text-sm text-text-muted">{description}</p>}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Copy Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button onClick={handleCopy} variant="ghost" size="sm" className="h-8 w-8 p-0">
                {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
              </Button>
            </TooltipTrigger>
            <TooltipContent>Copy selected file</TooltipContent>
          </Tooltip>

          {/* Preview Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                onClick={() => setIsPreviewOpen(!isPreviewOpen)}
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
              >
                {isPreviewOpen ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{isPreviewOpen ? 'Hide preview' : 'Show preview'}</TooltipContent>
          </Tooltip>

          {/* Download Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button onClick={handleDownload} variant="ghost" size="sm" className="h-8 w-8 p-0">
                <Download className="h-4 w-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Download as ZIP</TooltipContent>
          </Tooltip>

          {/* Edit/Save Button */}
          {isEditing ? (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button onClick={handleSave} variant="ghost" size="sm" className="h-8 w-8 p-0">
                    {saved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{saved ? 'Saved!' : 'Save changes'}</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button onClick={handleCancel} variant="ghost" size="sm" className="h-8 w-8 p-0">
                    <X className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Cancel editing</TooltipContent>
              </Tooltip>
            </>
          ) : (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  onClick={() => setIsEditing(true)}
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 p-0"
                >
                  <Edit3 className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Edit project</TooltipContent>
            </Tooltip>
          )}

          {/* Delete Button */}
          {onDelete && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  onClick={onDelete}
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 p-0 text-red-500 hover:text-red-600"
                >
                  <X className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Delete project</TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>

      {/* File List and Content */}
      <div className="flex">
        {/* File List */}
        <div className="w-64 border-r border-border-default bg-background-muted">
          <div className="p-3 border-b border-border-default">
            <h4 className="font-medium text-text-default">Project Files</h4>
          </div>
          <div className="max-h-96 overflow-y-auto">
            {editedFiles.map((file, index) => (
              <button
                key={index}
                onClick={() => setSelectedFile(file)}
                className={`w-full text-left p-3 hover:bg-background-default transition-colors ${
                  selectedFile?.path === file.path
                    ? 'bg-background-default border-r-2 border-blue-500'
                    : ''
                }`}
              >
                <div className="flex items-center gap-2">
                  <File className="h-4 w-4 text-text-muted" />
                  <span className="text-sm font-mono text-text-default">{file.name}</span>
                </div>
                <div className="text-xs text-text-muted mt-1">{file.path}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Code Content */}
        <div className="flex-1">
          {selectedFile && (
            <div className="p-4">
              <div className="flex items-center justify-between mb-3">
                <h4 className="font-medium text-text-default">{selectedFile.name}</h4>
                <span className="text-xs text-text-muted uppercase">{selectedFile.language}</span>
              </div>
              {isEditing ? (
                <textarea
                  value={selectedFile.content}
                  onChange={(e) => {
                    const updatedFiles = editedFiles.map((f) =>
                      f.path === selectedFile.path ? { ...f, content: e.target.value } : f
                    );
                    setEditedFiles(updatedFiles);
                    setSelectedFile({ ...selectedFile, content: e.target.value });
                  }}
                  className="w-full h-64 p-3 font-mono text-sm bg-background-muted border border-border-default rounded resize-none focus:outline-none focus:border-blue-500"
                />
              ) : (
                <pre className="w-full h-64 p-3 font-mono text-sm bg-background-muted border border-border-default rounded overflow-auto">
                  <code>{selectedFile.content}</code>
                </pre>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Preview Panel */}
      {isPreviewOpen && (
        <div className="border-t border-border-default">
          <div className="p-3 border-b border-border-default bg-background-muted">
            <h4 className="font-medium text-text-default">Live Preview</h4>
          </div>
          <iframe
            ref={(el) => {
              if (el) {
                el.srcdoc = generatePreviewHTML();
              }
            }}
            className="w-full h-96 border-0"
            sandbox="allow-scripts allow-same-origin"
            title="Project Preview"
          />
        </div>
      )}
    </div>
  );
};

export default NextJSProjectArtifact;
