import React, { useState, useEffect, memo, lazy, Suspense } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkBreaks from 'remark-breaks';
import { wrapHTMLInCodeBlock } from '../utils/htmlSecurity';

const CodeBlock = lazy(() => import('./CodeBlock'));

interface CodeProps extends React.ClassAttributes<HTMLElement>, React.HTMLAttributes<HTMLElement> {
  inline?: boolean;
}

interface MarkdownContentProps {
  content: string;
  className?: string;
}

const LightweightCodeBlock = function LightweightCodeBlock({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    // This mimics the code block from react-syntax-highlighter, but without the actual highlighting,
    // just so we have a fallback that isn't too jarringly different while the highlighter lazy-loads
    <div
      style={{
        background: 'rgb(40, 44, 52)',
        color: 'rgb(171, 178, 191)',
        whiteSpace: 'pre',
        lineHeight: 1.5,
        tabSize: 2,
        padding: '1em',
        margin: '0px',
        overflow: 'auto',
        borderRadius: '0.3em',
      }}
    >
      <code className="break-all whitespace-pre-wrap" style={{ fontFamily: 'monospace' }}>
        {children}
      </code>
    </div>
  );
};

const MarkdownCode = memo(
  React.forwardRef(function MarkdownCode(
    { inline, className, children, ...props }: CodeProps,
    ref: React.Ref<HTMLElement>
  ) {
    const match = /language-(\w+)/.exec(className || '');
    return !inline && match ? (
      <Suspense fallback={<LightweightCodeBlock>{children}</LightweightCodeBlock>}>
        <CodeBlock language={match[1]}>{String(children).replace(/\n$/, '')}</CodeBlock>
      </Suspense>
    ) : (
      <code ref={ref} {...props} className="break-all bg-inline-code whitespace-pre-wrap font-mono">
        {children}
      </code>
    );
  })
);

const MarkdownContent = memo(function MarkdownContent({
  content,
  className = '',
}: MarkdownContentProps) {
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
      className={`w-full overflow-x-hidden prose prose-sm text-text-default dark:prose-invert max-w-full word-break font-sans
      prose-pre:p-0 prose-pre:m-0 !p-0
      prose-code:break-all prose-code:whitespace-pre-wrap prose-code:font-sans
      prose-a:break-all prose-a:overflow-wrap-anywhere
      prose-table:table prose-table:w-full
      prose-blockquote:text-inherit
      prose-td:border prose-td:border-border-default prose-td:p-2
      prose-th:border prose-th:border-border-default prose-th:p-2
      prose-thead:bg-background-default
      prose-h1:text-2xl prose-h1:font-normal prose-h1:mb-5 prose-h1:mt-0 prose-h1:font-sans
      prose-h2:text-xl prose-h2:font-normal prose-h2:mb-4 prose-h2:mt-4 prose-h2:font-sans
      prose-h3:text-lg prose-h3:font-normal prose-h3:mb-3 prose-h3:mt-3 prose-h3:font-sans
      prose-p:mt-0 prose-p:mb-2 prose-p:font-sans
      prose-ol:my-2 prose-ol:font-sans
      prose-ul:mt-0 prose-ul:mb-3 prose-ul:font-sans
      prose-li:m-0 prose-li:font-sans ${className}`}
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
});

export default MarkdownContent;
