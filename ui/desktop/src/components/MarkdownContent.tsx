import React, { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { Check, Copy } from './icons';

interface CodeProps extends React.ClassAttributes<HTMLElement>, React.HTMLAttributes<HTMLElement> {
  inline?: boolean;
}

interface MarkdownContentProps {
  content: string;
  className?: string;
}

const CodeBlock = ({ language, children }: { language: string; children: string }) => {
  const [copied, setCopied] = useState(false);

  const [isCreating, setIsCreating] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  const handleCreateArtifact = async () => {
    setIsCreating(true);

    // Create a new artifact with the current code
    const artifactData = {
      title: `Generated ${language.toUpperCase()} Code`,
      language: language,
      description: `Code generated on ${new Date().toLocaleDateString()}`,
      code: children,
    };

    // Store in localStorage for now (in a real app, this would use the hook)
    try {
      const existingArtifacts = JSON.parse(
        localStorage.getItem('Goose-code-artifacts') || '[]'
      );
      const newArtifact = {
        id: Date.now().toString(36) + Math.random().toString(36).substr(2),
        ...artifactData,
        createdAt: new Date(),
        updatedAt: new Date(),
      };
      existingArtifacts.unshift(newArtifact);
      localStorage.setItem('Goose-code-artifacts', JSON.stringify(existingArtifacts));

      // Show success message and offer to open artifacts
      const shouldOpen = window.confirm(
        `Code artifact "${artifactData.title}" wurde erfolgreich erstellt!\n\nMöchtest du die Code Artifacts öffnen?`
      );

      if (shouldOpen) {
        // Navigate to code artifacts view
        window.location.hash = '#/code-artifacts';
      }
    } catch (error) {
      console.error('Failed to create artifact:', error);
      window.alert('Fehler beim Erstellen des Artifacts');
    } finally {
      setIsCreating(false);
    }
  };

  const isArtifactable = [
    'html',
    'css',
    'javascript',
    'jsx',
    'tsx',
    'typescript',
    'python',
    'java',
    'cpp',
    'csharp',
    'json',
    'yaml',
    'yml',
    'bash',
    'shell',
    'sh',
    'zsh',
    'powershell',
    'ps1',
    'markdown',
    'md',
    'txt',
    'text',
    'sql',
    'php',
    'ruby',
    'go',
    'rust',
    'swift',
    'kotlin',
    'scala',
    'r',
    'matlab',
    'dockerfile',
    'docker',
    'nginx',
    'apache',
    'conf',
    'xml',
    'svg',
    'graphql',
    'gql',
    'prisma',
    'toml',
    'ini',
    'env',
    'properties',
    'gitignore',
    'gitattributes',
  ].includes(language);

  return (
    <div className="relative group w-full">
      <div className="absolute right-2 bottom-2 flex gap-1 z-10">
        {/* Copy Button */}
        <button
          onClick={handleCopy}
          className="p-1.5 rounded-lg bg-gray-700/50 text-gray-300
                   opacity-0 group-hover:opacity-100 transition-opacity duration-200
                   hover:bg-gray-600/50 hover:text-gray-100"
          title="Copy code"
        >
          {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
        </button>

        {/* Create Artifact Button */}
        {isArtifactable && (
          <button
            onClick={handleCreateArtifact}
            disabled={isCreating}
            className="p-1.5 rounded-lg bg-blue-600/50 text-white
                              opacity-0 group-hover:opacity-100 transition-opacity duration-200
                              hover:bg-blue-500/50 disabled:opacity-50 disabled:cursor-not-allowed"
            title={isCreating ? 'Creating...' : 'Save as Code Artifact'}
          >
            {isCreating ? (
              <svg className="h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                ></circle>
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
            ) : (
              <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                />
              </svg>
            )}
          </button>
        )}
      </div>

      <div className="w-full overflow-x-auto">
        <SyntaxHighlighter
          style={oneDark}
          language={language}
          PreTag="div"
          customStyle={{
            margin: 0,
            width: '100%',
            maxWidth: '100%',
          }}
          codeTagProps={{
            style: {
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              overflowWrap: 'break-word',
            },
          }}
        >
          {children}
        </SyntaxHighlighter>
      </div>
    </div>
  );
};

const MarkdownCode = React.forwardRef(function MarkdownCode(
  { inline, className, children, ...props }: CodeProps,
  ref: React.Ref<HTMLElement>
) {
  const match = /language-(\w+)/.exec(className || '');
  return !inline && match ? (
    <CodeBlock language={match[1]}>{String(children).replace(/\n$/, '')}</CodeBlock>
  ) : (
    <code ref={ref} {...props} className="break-all bg-inline-code whitespace-pre-wrap">
      {children}
    </code>
  );
});

// Detect if content contains HTML
const containsHTML = (str: string) => {
  const htmlRegex = /<[^>]*>/;
  return htmlRegex.test(str);
};

// Wrap HTML content in code blocks
const wrapHTMLInCodeBlock = (content: string) => {
  if (containsHTML(content)) {
    // Split content by code blocks to preserve existing ones
    const parts = content.split(/(```[\s\S]*?```)/g);
    return parts
      .map((part) => {
        // If part is already a code block, leave it as is
        if (part.startsWith('```') && part.endsWith('```')) {
          return part;
        }
        // If part contains HTML, wrap it in HTML code block
        if (containsHTML(part)) {
          return `\`\`\`html\n${part}\n\`\`\``;
        }
        return part;
      })
      .join('\n');
  }
  return content;
};

export default function MarkdownContent({ content, className = '' }: MarkdownContentProps) {
  // Process content before rendering
  const processedContent = wrapHTMLInCodeBlock(content);

  return (
    <div className="w-full overflow-x-hidden">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        className={`prose prose-sm text-text-default dark:prose-invert w-full max-w-full word-break
          prose-pre:p-0 prose-pre:m-0 !p-0
          prose-code:break-all prose-code:whitespace-pre-wrap
          prose-table:table prose-table:w-full
          prose-blockquote:text-inherit
          prose-td:border prose-td:border-border-default prose-td:p-2
          prose-th:border prose-th:border-border-default prose-th:p-2
          prose-thead:bg-background-default
          prose-h1:text-2xl prose-h1:font-normal prose-h1:mb-5 prose-h1:mt-0
          prose-h2:text-xl prose-h2:font-normal prose-h2:mb-4 prose-h2:mt-4
          prose-h3:text-lg prose-h3:font-normal prose-h3:mb-3 prose-h3:mt-3
          prose-p:mt-0 prose-p:mb-2
          prose-ol:my-2
          prose-ul:mt-0 prose-ul:mb-3
          prose-li:m-0

          ${className}`}
        components={{
          a: ({ ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" />,
          code: MarkdownCode,
        }}
      >
        {processedContent}
      </ReactMarkdown>
    </div>
  );
}
