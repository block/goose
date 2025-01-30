import React, { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { oneLight } from 'react-syntax-highlighter/dist/cjs/styles/prism';
import { Check, Copy } from './icons';
import { visit } from 'unist-util-visit';
import { shell } from 'electron';

function rehypeinlineCodeProperty() {
  return function (tree) {
    if (!tree) return;
    visit(tree, 'element', function (node, index, parent) {
      if (node.tagName == 'code' && parent && parent.tagName === 'pre') {
        node.properties.inlinecode = 'false';
      } else {
        node.properties.inlinecode = 'true';
      }
    });
  };
}

interface MarkdownContentProps {
  content: string;
  className?: string;
}

const CodeBlock = ({ language, children }: { language: string; children: string }) => {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  return (
    <div className="relative group w-full">
      <button
        onClick={handleCopy}
        className="absolute right-2 bottom-2 p-1.5 rounded-lg bg-gray-700/50 text-gray-300
                 opacity-0 group-hover:opacity-100 transition-opacity duration-200
                 hover:bg-gray-600/50 hover:text-gray-100 z-10"
        title="Copy code"
      >
        {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
      </button>
      <div className="w-full overflow-x-auto">
        <SyntaxHighlighter
          style={oneDark}
          language={language}
          PreTag="div"
          customStyle={{
            margin: 0,
            width: '100%',
            maxWidth: '100%',
            minHeight: '3rem',
            padding: '1rem 0.75rem',
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

interface CustomLinkProps {
  href: string;
  children: React.ReactNode;
  title?: string;
  className?: string;
}

const CustomLink: React.FC<CustomLinkProps> = ({ href, children, title, className = '' }) => {
  const [error, setError] = useState<string | null>(null);

  const handleClick = async (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    if (!href) {
      setError('Invalid link');
      return;
    }

    try {
      await shell.openExternal(href);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      console.error('Failed to open link:', errorMessage);
      setError(`Failed to open link: ${errorMessage}`);
      setTimeout(() => setError(null), 3000);
    }
  };

  return (
    <span className="relative group">
      <a
        href={href}
        onClick={handleClick}
        title={title}
        className={`text-blue-500 hover:underline ${error ? 'cursor-not-allowed text-red-500' : ''}`}
      >
        {children}
      </a>
      {error && (
        <span className="absolute -bottom-6 left-0 text-xs text-red-500 bg-red-100 dark:bg-red-900/50 px-2 py-1 rounded">
          {error}
        </span>
      )}
    </span>
  );
};

export default function MarkdownContent({ content, className = '' }: MarkdownContentProps) {
  // Determine whether dark mode is enabled
  const isDarkMode = document.documentElement.classList.contains('dark');
  return (
    <div className="w-full overflow-x-hidden">
      <ReactMarkdown
        rehypePlugins={[rehypeinlineCodeProperty]}
        className={`prose prose-xs dark:prose-invert w-full max-w-full break-words
          prose-pre:p-0 prose-pre:m-0 prose-pre:min-h-[3rem]
          prose-code:break-all prose-code:whitespace-pre-wrap
          ${className}`}
        components={{
          code({ node, className, children, inlinecode, ...props }) {
            const match = /language-(\w+)/.exec(className || 'language-text');
            return inlinecode == 'false' && match ? (
              <CodeBlock language={match[1]}>{String(children).replace(/\n$/, '')}</CodeBlock>
            ) : (
              <code
                {...props}
                className={`${className} break-all bg-inline-code dark:bg-inline-code-dark whitespace-pre-wrap`}
              >
                {children}
              </code>
            );
          },
          a: CustomLink,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
