import React, { useRef, useEffect, useMemo } from 'react';
import Editor, { OnMount, OnChange } from '@monaco-editor/react';
import type * as monaco from 'monaco-editor';

interface MonacoCodeInputProps {
  language: string;
  value: string;
  onChange: (value: string) => void;
  onSend?: () => void;
  onExit?: () => void;
  height?: string | number;
  theme?: 'vs-dark' | 'light';
  className?: string;
}

export const MonacoCodeInput: React.FC<MonacoCodeInputProps> = ({
  language,
  value,
  onChange,
  onSend,
  onExit,
  height = 'auto',
  theme = 'vs-dark',
  className = '',
}) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof monaco | null>(null);

  const handleEditorDidMount: OnMount = (editor, monacoInstance) => {
    editorRef.current = editor;
    monacoRef.current = monacoInstance;
    
    // Focus the editor
    editor.focus();

    // Add Cmd/Ctrl+Enter to send
    editor.addCommand(
      monacoInstance.KeyMod.CtrlCmd | monacoInstance.KeyCode.Enter,
      () => {
        if (onSend) {
          onSend();
        }
      }
    );

    // Add Escape to exit code mode
    editor.addCommand(
      monacoInstance.KeyCode.Escape,
      () => {
        if (onExit) {
          onExit();
        }
      }
    );

    // Position cursor at end
    const model = editor.getModel();
    if (model) {
      const lineCount = model.getLineCount();
      const lastLineLength = model.getLineLength(lineCount);
      editor.setPosition({ lineNumber: lineCount, column: lastLineLength + 1 });
    }
  };

  const handleEditorChange: OnChange = (newValue) => {
    onChange(newValue || '');
  };

  // Calculate height based on line count
  const calculatedHeight = useMemo(() => {
    if (typeof height === 'number' || height !== 'auto') {
      return height;
    }
    
    const lines = value.split('\n').length;
    const lineHeight = 21;
    const padding = 16;
    const minHeight = 100;
    const maxHeight = 400;
    
    const contentHeight = lines * lineHeight + padding;
    return Math.min(Math.max(contentHeight, minHeight), maxHeight);
  }, [value, height]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (editorRef.current) {
        editorRef.current.dispose();
      }
    };
  }, []);

  const editorOptions: monaco.editor.IStandaloneEditorConstructionOptions = {
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 14,
    fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, source-code-pro, monospace',
    lineNumbers: 'on',
    renderLineHighlight: 'line',
    automaticLayout: true,
    wordWrap: 'on',
    wrappingStrategy: 'advanced',
    padding: { top: 8, bottom: 8 },
    suggest: {
      showKeywords: true,
      showSnippets: true,
    },
    quickSuggestions: {
      other: true,
      comments: false,
      strings: false,
    },
    tabSize: 2,
    insertSpaces: true,
    detectIndentation: true,
    folding: true,
    foldingStrategy: 'indentation',
    showFoldingControls: 'mouseover',
    matchBrackets: 'always',
    autoClosingBrackets: 'always',
    autoClosingQuotes: 'always',
    formatOnPaste: true,
    formatOnType: true,
    scrollbar: {
      vertical: 'auto',
      horizontal: 'auto',
      useShadows: false,
      verticalScrollbarSize: 10,
      horizontalScrollbarSize: 10,
    },
  };

  return (
    <div className={`monaco-code-input-wrapper ${className}`}>
      <Editor
        height={calculatedHeight}
        language={language}
        value={value}
        theme={theme}
        options={editorOptions}
        onMount={handleEditorDidMount}
        onChange={handleEditorChange}
        loading={
          <div className="flex items-center justify-center h-32">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500" />
          </div>
        }
      />
    </div>
  );
};

export default MonacoCodeInput;
