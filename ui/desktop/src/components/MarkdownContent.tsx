import React, { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { oneLight } from 'react-syntax-highlighter/dist/cjs/styles/prism';
import { Check, Copy } from './icons';
import { visit } from 'unist-util-visit';

const UrlTransform = {
  a: ({ node, ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" />,
};

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

export default function MarkdownContent({ content, className = '' }: MarkdownContentProps) {
  // Determine whether dark mode is enabled
  const isDarkMode = document.documentElement.classList.contains('dark');
  return (
    <div className="w-full overflow-x-hidden">
      <ReactMarkdown
        rehypePlugins={[rehypeinlineCodeProperty]}
        className={`prose prose-xs dark:prose-invert w-full max-w-full break-words
          prose-pre:p-0 prose-pre:m-0 !p-0
          prose-code:break-all prose-code:whitespace-pre-wrap
          ${className}`}
        components={{
          ...UrlTransform,
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
          // h3: 'div',
          h3(props) {
            const { node, ...rest } = props;
            return <div className="text-textStandard text-sm" {...rest} />;
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
