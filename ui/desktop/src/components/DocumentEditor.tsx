import React, { useState, useEffect, useCallback } from 'react';
import { useEditor, EditorContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import Underline from '@tiptap/extension-underline';
import Link from '@tiptap/extension-link';
import Image from '@tiptap/extension-image';
import Table from '@tiptap/extension-table';
import TableRow from '@tiptap/extension-table-row';
import TableCell from '@tiptap/extension-table-cell';
import TableHeader from '@tiptap/extension-table-header';
import {
  Bold,
  Italic,
  Underline as UnderlineIcon,
  Strikethrough,
  Code,
  Heading1,
  Heading2,
  Heading3,
  List,
  ListOrdered,
  Quote,
  Link as LinkIcon,
  Image as ImageIcon,
  Table as TableIcon,
  Save,
  SaveAll,
  FileText,
  AlertCircle,
  Loader2,
  Undo,
  Redo,
  ChevronDown,
} from 'lucide-react';
import { Button } from './ui/button';
import { usePersistence } from '../hooks/usePersistence';

interface DocumentEditorProps {
  filePath?: string;
  initialContent?: string;
  placeholder?: string;
  onSave?: (content: string, filePath?: string) => Promise<void>;
  readOnly?: boolean;
}

interface FileReadResult {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

export const DocumentEditor: React.FC<DocumentEditorProps> = ({
  filePath,
  initialContent = '',
  placeholder = 'Start writing...',
  onSave,
  readOnly = false,
}) => {
  const [content, setContent] = useState(initialContent);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Use the persistence hook
  const persistence = usePersistence({
    filePath,
    autoSave: true,
    autoSaveInterval: 30000, // 30 seconds
    onSave: (result) => {
      if (result.success) {
        console.log('Document saved successfully to:', result.filePath);
      } else {
        setError(result.error || 'Save failed');
      }
    },
    onLoad: (result) => {
      if (result.success && result.content) {
        setContent(result.content);
        editor?.commands.setContent(result.content);
      } else {
        setError(result.error || 'Load failed');
      }
    },
    onError: (errorMessage) => {
      setError(errorMessage);
    },
  });

  const editor = useEditor({
    extensions: [
      StarterKit,
      Placeholder.configure({
        placeholder,
      }),
      Underline,
      Link.configure({
        openOnClick: false,
        HTMLAttributes: {
          class: 'text-blue-500 underline cursor-pointer',
        },
      }),
      Image.configure({
        HTMLAttributes: {
          class: 'max-w-full h-auto rounded-lg',
        },
      }),
      Table.configure({
        resizable: true,
      }),
      TableRow,
      TableHeader,
      TableCell,
    ],
    content: content,
    editable: !readOnly,
    onUpdate: ({ editor }) => {
      const newContent = editor.getHTML();
      setContent(newContent);
      persistence.updateContent(newContent);
    },
  });

  // Load file content if filePath is provided
  useEffect(() => {
    const loadFile = async () => {
      if (!filePath) return;

      setIsLoading(true);
      setError(null);

      try {
        console.log('Loading document:', filePath);
        
        // Use Electron's readFile API
        const result = await window.electron.readFile(filePath) as FileReadResult;
        
        if (result.found && result.error === null) {
          const fileContent = result.file;
          
          // Detect file type and convert to HTML if needed
          let htmlContent = fileContent;
          
          if (filePath.endsWith('.md') || filePath.endsWith('.markdown')) {
            // Convert Markdown to HTML (basic conversion)
            htmlContent = markdownToHtml(fileContent);
          } else if (filePath.endsWith('.txt')) {
            // Convert plain text to HTML
            htmlContent = `<p>${fileContent.replace(/\n/g, '</p><p>')}</p>`;
          } else if (!filePath.endsWith('.html')) {
            // For other file types, treat as plain text
            htmlContent = `<pre><code>${fileContent}</code></pre>`;
          }
          
          setContent(htmlContent);
          editor?.commands.setContent(htmlContent);
          persistence.markAsSaved();
        } else {
          const errorMessage = result.error || 'File not found';
          setError(errorMessage);
        }
      } catch (err) {
        console.error('Error loading document:', err);
        setError(err instanceof Error ? err.message : 'Failed to load document');
      } finally {
        setIsLoading(false);
      }
    };

    loadFile();
  }, [filePath, editor]);

  // Basic Markdown to HTML converter
  const markdownToHtml = (markdown: string): string => {
    return markdown
      .replace(/^# (.*$)/gm, '<h1>$1</h1>')
      .replace(/^## (.*$)/gm, '<h2>$1</h2>')
      .replace(/^### (.*$)/gm, '<h3>$1</h3>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      .replace(/`(.*?)`/g, '<code>$1</code>')
      .replace(/\n/g, '<br>');
  };

  // Convert HTML back to appropriate format for saving
  const htmlToFileFormat = (html: string, targetPath: string): string => {
    if (targetPath.endsWith('.md') || targetPath.endsWith('.markdown')) {
      // Convert HTML back to Markdown (basic conversion)
      return html
        .replace(/<h1>(.*?)<\/h1>/g, '# $1')
        .replace(/<h2>(.*?)<\/h2>/g, '## $1')
        .replace(/<h3>(.*?)<\/h3>/g, '### $1')
        .replace(/<strong>(.*?)<\/strong>/g, '**$1**')
        .replace(/<em>(.*?)<\/em>/g, '*$1*')
        .replace(/<code>(.*?)<\/code>/g, '`$1`')
        .replace(/<br>/g, '\n')
        .replace(/<p>(.*?)<\/p>/g, '$1\n\n');
    } else if (targetPath.endsWith('.txt')) {
      // Strip HTML tags for plain text
      return html.replace(/<[^>]*>/g, '').replace(/\n\n+/g, '\n\n');
    }
    
    // Return HTML for .html files or unknown formats
    return html;
  };

  // The persistence hook already handles saving and keyboard shortcuts

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center bg-background-default">
        <div className="flex items-center space-x-2 text-text-muted">
          <Loader2 className="w-5 h-5 animate-spin" />
          <span>Loading document...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center bg-background-default">
        <div className="flex flex-col items-center space-y-2 text-text-muted">
          <AlertCircle className="w-8 h-8 text-red-500" />
          <span className="text-sm">Error loading document</span>
          <span className="text-xs text-text-subtle">{error}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-background-default">
      {/* Toolbar */}
      {!readOnly && (
        <div className="flex-shrink-0 border-b border-border-subtle bg-background-muted p-2">
          <div className="flex items-center space-x-1 flex-wrap gap-1">
            {/* Save button */}
            <Button
              onClick={() => persistence.save()}
              disabled={!persistence.hasUnsavedChanges || persistence.isSaving}
              variant="ghost"
              size="sm"
              className="flex items-center space-x-1"
            >
              {persistence.isSaving ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Save className="w-4 h-4" />
              )}
              <span className="text-xs">Save</span>
            </Button>

            {/* Save As button */}
            <Button
              onClick={() => persistence.saveAs()}
              disabled={persistence.isSaving}
              variant="ghost"
              size="sm"
              className="flex items-center space-x-1"
            >
              <SaveAll className="w-4 h-4" />
              <span className="text-xs">Save As</span>
            </Button>

            <div className="w-px h-6 bg-border-subtle mx-1" />

            {/* Undo/Redo */}
            <Button
              onClick={() => editor?.chain().focus().undo().run()}
              disabled={!editor?.can().undo()}
              variant="ghost"
              size="sm"
            >
              <Undo className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().redo().run()}
              disabled={!editor?.can().redo()}
              variant="ghost"
              size="sm"
            >
              <Redo className="w-4 h-4" />
            </Button>

            <div className="w-px h-6 bg-border-subtle mx-1" />

            {/* Text formatting */}
            <Button
              onClick={() => editor?.chain().focus().toggleBold().run()}
              variant={editor?.isActive('bold') ? 'default' : 'ghost'}
              size="sm"
            >
              <Bold className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleItalic().run()}
              variant={editor?.isActive('italic') ? 'default' : 'ghost'}
              size="sm"
            >
              <Italic className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleUnderline().run()}
              variant={editor?.isActive('underline') ? 'default' : 'ghost'}
              size="sm"
            >
              <UnderlineIcon className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleStrike().run()}
              variant={editor?.isActive('strike') ? 'default' : 'ghost'}
              size="sm"
            >
              <Strikethrough className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleCode().run()}
              variant={editor?.isActive('code') ? 'default' : 'ghost'}
              size="sm"
            >
              <Code className="w-4 h-4" />
            </Button>

            <div className="w-px h-6 bg-border-subtle mx-1" />

            {/* Headings */}
            <Button
              onClick={() => editor?.chain().focus().toggleHeading({ level: 1 }).run()}
              variant={editor?.isActive('heading', { level: 1 }) ? 'default' : 'ghost'}
              size="sm"
            >
              <Heading1 className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()}
              variant={editor?.isActive('heading', { level: 2 }) ? 'default' : 'ghost'}
              size="sm"
            >
              <Heading2 className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleHeading({ level: 3 }).run()}
              variant={editor?.isActive('heading', { level: 3 }) ? 'default' : 'ghost'}
              size="sm"
            >
              <Heading3 className="w-4 h-4" />
            </Button>

            <div className="w-px h-6 bg-border-subtle mx-1" />

            {/* Lists */}
            <Button
              onClick={() => editor?.chain().focus().toggleBulletList().run()}
              variant={editor?.isActive('bulletList') ? 'default' : 'ghost'}
              size="sm"
            >
              <List className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleOrderedList().run()}
              variant={editor?.isActive('orderedList') ? 'default' : 'ghost'}
              size="sm"
            >
              <ListOrdered className="w-4 h-4" />
            </Button>
            <Button
              onClick={() => editor?.chain().focus().toggleBlockquote().run()}
              variant={editor?.isActive('blockquote') ? 'default' : 'ghost'}
              size="sm"
            >
              <Quote className="w-4 h-4" />
            </Button>
          </div>
        </div>
      )}

      {/* Document header */}
      <div className="flex-shrink-0 px-4 py-2 border-b border-border-subtle bg-background-muted">
        <div className="flex items-center space-x-2">
          <FileText className="w-4 h-4 text-text-muted" />
          <span className="text-sm font-mono text-text-standard truncate">
            {persistence.filePath ? persistence.filePath.split('/').pop() || persistence.filePath : 'Untitled Document'}
          </span>
          {persistence.hasUnsavedChanges && (
            <span className="text-xs text-orange-500 bg-orange-100 dark:bg-orange-900/30 px-2 py-1 rounded">
              Unsaved
            </span>
          )}
          {persistence.isAutoSaving && (
            <span className="text-xs text-blue-500 bg-blue-100 dark:bg-blue-900/30 px-2 py-1 rounded">
              Auto-saving...
            </span>
          )}
          {readOnly && (
            <span className="text-xs text-text-subtle bg-background-default px-2 py-1 rounded">
              Read Only
            </span>
          )}
        </div>
        {persistence.filePath && (
          <div className="text-xs text-text-subtle mt-1 font-mono truncate">
            {persistence.filePath}
          </div>
        )}
      </div>

      {/* Editor content */}
      <div className="flex-1 overflow-auto">
        <div className="p-4">
          <EditorContent
            editor={editor}
            className="prose prose-sm max-w-none focus:outline-none"
          />
        </div>
      </div>

      {/* Status bar */}
      <div className="flex-shrink-0 px-4 py-2 border-t border-border-subtle bg-background-muted text-xs text-text-subtle">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <span>
              {editor?.storage.characterCount?.characters() || 0} characters
            </span>
            <span>
              {editor?.storage.characterCount?.words() || 0} words
            </span>
          </div>
          {persistence.hasUnsavedChanges && (
            <span className="text-orange-500">
              Press Ctrl+S to save
            </span>
          )}
        </div>
      </div>
    </div>
  );
};

export default DocumentEditor;
