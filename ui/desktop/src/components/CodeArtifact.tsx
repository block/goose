import React, { useState, useRef } from 'react';
import { Download, Copy, Check, X, Edit3, Save, Eye, EyeOff } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';

interface CodeArtifactProps {
  code: string;
  language: string;
  title?: string;
  description?: string;
  onSave?: (code: string, title: string) => void;
  onDelete?: () => void;
}

export const CodeArtifact: React.FC<CodeArtifactProps> = ({
  code,
  language,
  title = 'Generated Code',
  description,
  onSave,
  onDelete,
}) => {
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editedCode, setEditedCode] = useState(code);
  const [editedTitle, setEditedTitle] = useState(title);
  const [copied, setCopied] = useState(false);
  const [saved, setSaved] = useState(false);
  // eslint-disable-next-line no-undef
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  // Generate preview HTML for HTML/CSS/JS code
  const generatePreviewHTML = (code: string, lang: string) => {
    if (lang === 'html') {
      // Check if it's a complete HTML document or just HTML fragments
      if (
        code.trim().toLowerCase().startsWith('<!doctype html>') ||
        code.trim().toLowerCase().startsWith('<html')
      ) {
        return code;
      } else {
        // Wrap HTML fragments in a complete document
        return `
          <!DOCTYPE html>
          <html lang="en">
          <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>HTML Preview</title>
            <script src="https://cdn.tailwindcss.com"></script>
            <style>
              body { font-family: Arial, sans-serif; margin: 0; padding: 20px; }
              * { box-sizing: border-box; }
            </style>
          </head>
          <body>
            ${code}
          </body>
          </html>
        `;
      }
    } else if (lang === 'css') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>CSS Preview</title>
          <style>${code}</style>
        </head>
        <body>
          <div style="padding: 20px; min-height: 100vh;">
            <h1>CSS Preview</h1>
            <p>Hier siehst du deine CSS-Styles angewendet:</p>
            
            <div class="demo-section">
              <h2>Buttons</h2>
              <button class="btn">Normal Button</button>
              <button class="btn primary">Primary Button</button>
              <button class="btn secondary">Secondary Button</button>
            </div>
            
            <div class="demo-section">
              <h2>Cards</h2>
              <div class="card">
                <h3>Sample Card</h3>
                <p>Dies ist ein Beispiel für eine Karte mit deinen CSS-Styles.</p>
              </div>
            </div>
            
            <div class="demo-section">
              <h2>Navigation</h2>
              <nav class="nav">
                <a href="#" class="nav-link">Home</a>
                <a href="#" class="nav-link">About</a>
                <a href="#" class="nav-link">Contact</a>
              </nav>
            </div>
            
            <div class="demo-section">
              <h2>Form Elements</h2>
              <input type="text" placeholder="Text Input" class="input">
              <select class="select">
                <option>Option 1</option>
                <option>Option 2</option>
                <option>Option 3</option>
              </select>
            </div>
          </div>
        </body>
        </html>
      `;
    } else if (lang === 'javascript') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>JavaScript Preview</title>
          <script src="https://cdn.tailwindcss.com"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .console { background: #1e1e1e; color: #fff; padding: 15px; border-radius: 4px; font-family: 'Courier New', monospace; margin-top: 20px; max-height: 200px; overflow-y: auto; }
            .button { background: #007bff; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; margin: 5px; }
            .button:hover { background: #0056b3; }
            .output { margin-top: 15px; padding: 10px; background: #f8f9fa; border-radius: 4px; border-left: 4px solid #007bff; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>JavaScript Preview</h1>
            <p>Klicke auf "Code ausführen" um deinen JavaScript-Code zu testen.</p>
            
            <div>
              <button onclick="runCode()" class="button">Code ausführen</button>
              <button onclick="clearConsole()" class="button">Console leeren</button>
            </div>
            
            <div id="output" class="output" style="display: none;">
              <strong>Output:</strong>
              <div id="output-content"></div>
            </div>
            
            <div class="console">
              <div><strong>Console Output:</strong></div>
              <div id="console-output"></div>
            </div>
          </div>
          
          <script>
            // Override console methods to capture output
            const originalLog = console.log;
            const originalError = console.error;
            const originalWarn = console.warn;
            const originalInfo = console.info;
            
            function addToConsole(message, type = 'log') {
              const consoleOutput = document.getElementById('console-output');
              const timestamp = new Date().toLocaleTimeString();
              const color = type === 'error' ? '#ff6b6b' : type === 'warn' ? '#ffd93d' : type === 'info' ? '#4ecdc4' : '#fff';
              consoleOutput.innerHTML += \`<div style="color: \${color};">[\${timestamp}] \${message}</div>\`;
              consoleOutput.scrollTop = consoleOutput.scrollHeight;
            }
            
            console.log = function(...args) {
              originalLog.apply(console, args);
              addToConsole(args.join(' '), 'log');
            };
            
            console.error = function(...args) {
              originalError.apply(console, args);
              addToConsole(args.join(' '), 'error');
            };
            
            console.warn = function(...args) {
              originalWarn.apply(console, args);
              addToConsole(args.join(' '), 'warn');
            };
            
            console.info = function(...args) {
              originalInfo.apply(console, args);
              addToConsole(args.join(' '), 'info');
            };
            
            function runCode() {
              try {
                // Clear previous output
                document.getElementById('output').style.display = 'none';
                document.getElementById('console-output').innerHTML = '';
                
                // Execute the code
                ${code}
                
                // Show output area
                document.getElementById('output').style.display = 'block';
                document.getElementById('output-content').innerHTML = 'Code erfolgreich ausgeführt!';
              } catch (error) {
                console.error('Error:', error);
                document.getElementById('output').style.display = 'block';
                document.getElementById('output-content').innerHTML = '<span style="color: red;">Error: ' + error.message + '</span>';
              }
            }
            
            function clearConsole() {
              document.getElementById('console-output').innerHTML = '';
              document.getElementById('output').style.display = 'none';
            }
          </script>
        </body>
        </html>
      `;
    } else if (lang === 'jsx' || lang === 'tsx' || lang === 'typescript') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>React Component Preview</title>
          <script src="https://unpkg.com/react@18/umd/react.development.js"></script>
          <script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
          <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .error { color: red; background: #ffe6e6; padding: 10px; border-radius: 4px; margin: 10px 0; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>React Component Preview</h1>
            <div id="root"></div>
            <div id="error-container"></div>
          </div>
          
          <script type="text/babel">
            try {
              ${code}
              
              // Try to render the component
              const rootElement = document.getElementById('root');
              const errorContainer = document.getElementById('error-container');
              
              // Check if App component exists
              if (typeof App !== 'undefined') {
                ReactDOM.render(React.createElement(App), rootElement);
              } else {
                // If no App component, try to render the first exported component
                const componentNames = Object.keys(window).filter(key => 
                  typeof window[key] === 'function' && 
                  key[0] === key[0].toUpperCase()
                );
                
                if (componentNames.length > 0) {
                  ReactDOM.render(React.createElement(window[componentNames[0]]), rootElement);
                } else {
                  errorContainer.innerHTML = '<div class="error">Keine React-Komponente gefunden. Stelle sicher, dass eine Komponente namens "App" oder eine andere Komponente exportiert wird.</div>';
                }
              }
            } catch (error) {
              document.getElementById('error-container').innerHTML = \`<div class="error">Fehler beim Rendern der React-Komponente: \${error.message}</div>\`;
              console.error('React rendering error:', error);
            }
          </script>
        </body>
        </html>
      `;
    } else if (lang === 'json') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>JSON Preview</title>
          <script src="https://cdn.tailwindcss.com"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .json-viewer { background: #1e1e1e; color: #fff; padding: 15px; border-radius: 4px; font-family: 'Courier New', monospace; overflow-x: auto; }
            .json-key { color: #9cdcfe; }
            .json-string { color: #ce9178; }
            .json-number { color: #b5cea8; }
            .json-boolean { color: #569cd6; }
            .json-null { color: #569cd6; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>JSON Preview</h1>
            <div class="json-viewer" id="json-display"></div>
          </div>
          <script>
            try {
              const jsonData = ${code};
              const jsonDisplay = document.getElementById('json-display');
              jsonDisplay.textContent = JSON.stringify(jsonData, null, 2);
            } catch (error) {
              document.getElementById('json-display').innerHTML = '<span style="color: #ff6b6b;">Invalid JSON: ' + error.message + '</span>';
            }
          </script>
        </body>
        </html>
      `;
    } else if (lang === 'yaml' || lang === 'yml') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>YAML Preview</title>
          <script src="https://cdn.tailwindcss.com"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .yaml-viewer { background: #1e1e1e; color: #fff; padding: 15px; border-radius: 4px; font-family: 'Courier New', monospace; white-space: pre-wrap; overflow-x: auto; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>YAML Preview</h1>
            <div class="yaml-viewer">${code.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</div>
          </div>
        </body>
        </html>
      `;
    } else if (lang === 'markdown' || lang === 'md') {
      return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Markdown Preview</title>
          <script src="https://cdn.tailwindcss.com"></script>
          <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
          <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
            .preview-container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
            .markdown-content { line-height: 1.6; }
            .markdown-content h1, .markdown-content h2, .markdown-content h3 { margin-top: 1.5em; margin-bottom: 0.5em; }
            .markdown-content p { margin-bottom: 1em; }
            .markdown-content code { background: #f1f1f1; padding: 2px 4px; border-radius: 3px; }
            .markdown-content pre { background: #f1f1f1; padding: 15px; border-radius: 4px; overflow-x: auto; }
            .markdown-content blockquote { border-left: 4px solid #ddd; padding-left: 1em; margin: 1em 0; }
          </style>
        </head>
        <body>
          <div class="preview-container">
            <h1>Markdown Preview</h1>
            <div class="markdown-content" id="markdown-display"></div>
          </div>
          <script>
            try {
              const markdownText = \`${code.replace(/`/g, '\\`')}\`;
              const htmlContent = marked.parse(markdownText);
              document.getElementById('markdown-display').innerHTML = htmlContent;
            } catch (error) {
              document.getElementById('markdown-display').innerHTML = '<span style="color: #ff6b6b;">Error rendering markdown: ' + error.message + '</span>';
            }
          </script>
        </body>
        </html>
      `;
    }
    return code;
  };

  const handlePreview = () => {
    if (isPreviewOpen) {
      setIsPreviewOpen(false);
    } else {
      setIsPreviewOpen(true);
    }
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(editedCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  };

  const handleDownload = () => {
    const extension =
      language === 'javascript'
        ? 'js'
        : language === 'typescript'
          ? 'ts'
          : language === 'jsx'
            ? 'jsx'
            : language === 'tsx'
              ? 'tsx'
              : language === 'css'
                ? 'css'
                : language === 'html'
                  ? 'html'
                  : 'txt';

    const blob = new Blob([editedCode], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${editedTitle || 'code'}.${extension}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleSave = () => {
    if (onSave) {
      onSave(editedCode, editedTitle);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    }
    setIsEditing(false);
  };

  const handleEdit = () => {
    setIsEditing(true);
  };

  const handleCancelEdit = () => {
    setEditedCode(code);
    setEditedTitle(title);
    setIsEditing(false);
  };

  const isPreviewable = [
    'html',
    'css',
    'javascript',
    'jsx',
    'tsx',
    'typescript',
    'json',
    'yaml',
    'yml',
    'markdown',
    'md',
    'txt',
    'text',
  ].includes(language);

  return (
    <div className="border border-border-default rounded-lg bg-background-default overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border-default bg-background-muted">
        <div className="flex-1">
          {isEditing ? (
            <input
              type="text"
              value={editedTitle}
              onChange={(e) => setEditedTitle(e.target.value)}
              className="w-full px-2 py-1 text-sm font-medium bg-background-default border border-border-default rounded"
              placeholder="Code title..."
            />
          ) : (
            <h3 className="text-sm font-medium text-text-default">{editedTitle}</h3>
          )}
          {description && <p className="text-xs text-text-muted mt-1">{description}</p>}
        </div>

        <div className="flex items-center gap-2">
          {/* Copy Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleCopy} className="h-8 w-8 p-0">
                {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
              </Button>
            </TooltipTrigger>
            <TooltipContent>Copy code</TooltipContent>
          </Tooltip>

          {/* Preview Button */}
          {isPreviewable && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="sm" onClick={handlePreview} className="h-8 w-8 p-0">
                  {isPreviewOpen ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </Button>
              </TooltipTrigger>
              <TooltipContent>Toggle preview</TooltipContent>
            </Tooltip>
          )}

          {/* Download Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleDownload} className="h-8 w-8 p-0">
                <Download className="h-4 w-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Download code</TooltipContent>
          </Tooltip>

          {/* Edit/Save Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={isEditing ? handleSave : handleEdit}
                className="h-8 w-8 p-0"
              >
                {isEditing ? (
                  saved ? (
                    <Check className="h-4 w-4" />
                  ) : (
                    <Save className="h-4 w-4" />
                  )
                ) : (
                  <Edit3 className="h-4 w-4" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{isEditing ? 'Save changes' : 'Edit code'}</TooltipContent>
          </Tooltip>

          {/* Cancel Edit Button */}
          {isEditing && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleCancelEdit}
                  className="h-8 w-8 p-0"
                >
                  <X className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Cancel edit</TooltipContent>
            </Tooltip>
          )}

          {/* Delete Button */}
          {onDelete && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onDelete}
                  className="h-8 w-8 p-0 text-red-500 hover:text-red-600"
                >
                  <X className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Delete artifact</TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>

      {/* Code Content */}
      <div className="p-4">
        {isEditing ? (
          <textarea
            value={editedCode}
            onChange={(e) => setEditedCode(e.target.value)}
            className="w-full h-64 p-3 font-mono text-sm bg-background-muted border border-border-default rounded resize-none"
            placeholder="Enter your code here..."
          />
        ) : (
          <pre className="w-full overflow-x-auto p-3 bg-background-muted border border-border-default rounded text-sm font-mono">
            <code>{editedCode}</code>
          </pre>
        )}
      </div>

      {/* Preview Panel */}
      {isPreviewOpen && isPreviewable && (
        <div className="border-t border-border-default">
          <div className="p-4 bg-background-muted border-b border-border-default">
            <h4 className="text-sm font-medium text-text-default">Live Preview</h4>
            <p className="text-xs text-text-muted mt-1">
              {language === 'html' && 'HTML content preview'}
              {language === 'css' && 'CSS styles applied to sample content'}
              {language === 'javascript' && 'JavaScript execution (check console)'}
              {(language === 'jsx' || language === 'tsx') && 'React component preview'}
            </p>
          </div>
          <div className="h-96 border-b border-border-default">
            <iframe
              ref={iframeRef}
              srcDoc={generatePreviewHTML(editedCode, language)}
              className="w-full h-full border-0"
              sandbox="allow-scripts allow-same-origin"
              title="Code preview"
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default CodeArtifact;
