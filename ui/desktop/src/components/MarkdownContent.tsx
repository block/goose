import React, { useState, useEffect, useRef } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkBreaks from 'remark-breaks';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { Check, Copy } from './icons';
import { wrapHTMLInCodeBlock } from '../utils/htmlSecurity';

interface CodeProps extends React.ClassAttributes<HTMLElement>, React.HTMLAttributes<HTMLElement> {
  inline?: boolean;
}

interface MarkdownContentProps {
  content: string;
  className?: string;
}

const CodeBlock = ({ language, children }: { language: string; children: string }) => {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);

      if (timeoutRef.current) {
        window.clearTimeout(timeoutRef.current);
      }

      timeoutRef.current = window.setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        window.clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return (
    <div className="relative group w-full">
      <button
        onClick={handleCopy}
        className="absolute right-2 bottom-2 p-1.5 rounded-lg bg-gray-700/50 text-gray-300 font-sans text-sm
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
              fontFamily: 'var(--font-sans)',
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
    <code ref={ref} {...props} className="break-all bg-inline-code whitespace-pre-wrap font-sans">
      {children}
    </code>
  );
});

export default function MarkdownContent({ content, className = '' }: MarkdownContentProps) {
  const [processedContent, setProcessedContent] = useState(content);

  useEffect(() => {
    try {
      const processed = wrapHTMLInCodeBlock(content);
      setProcessedContent(processed);
    } catch (error) {
      console.error('Error processing content:', error);
      // Fallback to original content if processing fails
      setProcessedContent(content);
    }
  }, [content]);

  return (
    <div
      className={`w-full overflow-x-hidden prose prose-sm text-text-default dark:prose-invert max-w-full word-breakfont-sans
      prose-pre:p-0 prose-pre:m-0 !p-0
      prose-code:break-all prose-code:whitespace-pre-wrapprose-code:font-sans
      prose-table:table prose-table:w-full
      prose-blockquote:text-inherit
      prose-td:border prose-td:border-border-default prose-td:p-2
      prose-th:border prose-th:border-border-default prose-th:p-2
      prose-thead:bg-background-default
      prose-h1:text-2xl prose-h1:font-normal prose-h1:mb-5 prose-h1:mt-0prose-h1:font-sans
      prose-h2:text-xl prose-h2:font-normal prose-h2:mb-4 prose-h2:mt-4prose-h2:font-sans
      prose-h3:text-lg prose-h3:font-normal prose-h3:mb-3 prose-h3:mt-3prose-h3:font-sans
      prose-p:mt-0 prose-p:mb-2prose-p:font-sans
      prose-ol:my-2prose-ol:font-sans
      prose-ul:mt-0 prose-ul:mb-3prose-ul:font-sans
      prose-li:m-0prose-li:font-sans ${className}`}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
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
