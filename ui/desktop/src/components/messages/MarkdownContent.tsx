import React, { memo, useEffect, useMemo, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeKatex from 'rehype-katex';
import remarkBreaks from 'remark-breaks';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import 'katex/dist/katex.min.css';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';

// Improved oneDark theme for better comment contrast and readability
const customOneDarkTheme = {
  ...oneDark,
  'code[class*="language-"]': {
    ...oneDark['code[class*="language-"]'],
    color: '#e6e6e6',
    fontSize: '14px',
  },
  'pre[class*="language-"]': {
    ...oneDark['pre[class*="language-"]'],
    color: '#e6e6e6',
    fontSize: '14px',
  },
  comment: { ...oneDark.comment, color: '#a0a0a0', fontStyle: 'italic' },
  prolog: { ...oneDark.prolog, color: '#a0a0a0' },
  doctype: { ...oneDark.doctype, color: '#a0a0a0' },
  cdata: { ...oneDark.cdata, color: '#a0a0a0' },
};

import { wrapHTMLInCodeBlock } from '../../utils/htmlSecurity';
import { wrapBareJsonRender } from '../../utils/jsonRenderDetection';
import { BLOCKED_PROTOCOLS, getProtocol, isProtocolSafe } from '../../utils/urlSecurity';
import { Check, Copy } from '../icons';
import { JsonRenderBlock } from '../json-render';

interface CodeProps extends React.ClassAttributes<HTMLElement>, React.HTMLAttributes<HTMLElement> {
  inline?: boolean;
}

interface MarkdownContentProps {
  content: string;
  className?: string;
}

// Memoized CodeBlock component to prevent re-rendering when props haven't changed
const CodeBlock = memo(function CodeBlock({
  language,
  children,
}: {
  language: string;
  children: string;
}) {
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

  // Memoize the SyntaxHighlighter component to prevent re-rendering
  // Only re-render if language or children change
  const memoizedSyntaxHighlighter = useMemo(() => {
    // For very large code blocks, consider truncating or lazy loading
    const isLargeCodeBlock = children.length > 10000; // 10KB threshold

    if (isLargeCodeBlock) {
    }

    return (
      <SyntaxHighlighter
        style={customOneDarkTheme}
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
            wordBreak: 'break-word',
            overflowWrap: 'anywhere',
            fontFamily: 'var(--font-mono)',
            fontSize: '14px',
          },
        }}
        // Performance optimizations for SyntaxHighlighter
        showLineNumbers={false} // Disable line numbers for better performance
        wrapLines={false} // Disable line wrapping for better performance
        lineProps={undefined} // Don't add extra props to each line
      >
        {children}
      </SyntaxHighlighter>
    );
  }, [language, children]);

  return (
    <div className="relative group w-full">
      <button
        type="button"
        onClick={handleCopy}
        className="absolute right-2 bottom-2 p-1.5 rounded-lg bg-gray-700/50 text-gray-300 font-sans text-sm
                 opacity-0 group-hover:opacity-100 transition-opacity duration-200
                 hover:bg-gray-600/50 hover:text-gray-100 z-10"
        title="Copy code"
      >
        {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
      </button>
      <div className="w-full overflow-x-auto">{memoizedSyntaxHighlighter}</div>
    </div>
  );
});

const MarkdownCode = memo(
  React.forwardRef(function MarkdownCode(
    { inline, className, children, ...props }: CodeProps,
    ref: React.Ref<HTMLElement>
  ) {
    const match = /language-([\w-]+)/.exec(className || '');
    if (!inline && match) {
      const lang = match[1];
      if (lang === 'json-render' || lang === 'jsonrender') {
        return (
          <div className="not-prose" data-json-render-block="true">
            <JsonRenderBlock spec={String(children).replace(/\n$/, '')} />
          </div>
        );
      }
      return <CodeBlock language={lang}>{String(children).replace(/\n$/, '')}</CodeBlock>;
    }
    return (
      <code
        ref={ref}
        {...props}
        className="break-words bg-[var(--inline-code-bg)] text-[var(--inline-code-fg)] whitespace-pre-wrap font-mono px-1 py-0.5 rounded"
      >
        {children}
      </code>
    );
  })
);

// Custom URL transform to preserve deep link URLs (spotify:, vscode:, slack:, etc.)
// React-markdown's default only allows http/https/mailto and strips all other protocols
// We allow all protocols except dangerous ones (javascript:, data:, file:, etc.)
const customUrlTransform = (url: string): string => {
  try {
    const protocol = new URL(url).protocol;
    if (BLOCKED_PROTOCOLS.includes(protocol)) {
      return '';
    }
  } catch {
    // Not a valid URL, allow it (could be relative path)
  }
  return url;
};

const MarkdownContent = memo(function MarkdownContent({
  content,
  className = '',
}: MarkdownContentProps) {
  const [processedContent, setProcessedContent] = useState(content);

  useEffect(() => {
    try {
      const processed = wrapHTMLInCodeBlock(content);
      setProcessedContent(wrapBareJsonRender(processed));
    } catch (error) {
      console.error('Error processing content:', error);
      setProcessedContent(content);
    }
  }, [content]);

  return (
    <div
      className={`w-full overflow-x-hidden prose prose-sm text-text-default dark:prose-invert max-w-full word-break font-sans
      prose-pre:p-0 prose-pre:m-0 !p-0
      prose-code:break-words prose-code:whitespace-pre-wrap prose-code:font-mono
      prose-pre:bg-transparent prose-pre:overflow-visible
      prose-a:break-words prose-a:overflow-wrap-anywhere
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
        urlTransform={customUrlTransform}
        remarkPlugins={[remarkGfm, remarkBreaks, [remarkMath, { singleDollarTextMath: false }]]}
        rehypePlugins={[
          [
            rehypeKatex,
            {
              throwOnError: false,
              errorColor: '#cc0000',
              strict: false,
            },
          ],
        ]}
        components={{
          pre: ({ children }) => {
            const childArray = React.Children.toArray(children);
            const elementChildren = childArray.filter((c): c is React.ReactElement =>
              React.isValidElement(c)
            );

            const jsonRenderChild = elementChildren.find((el) =>
              Boolean(
                (el.props as { ['data-json-render-block']?: unknown })?.['data-json-render-block']
              )
            );

            // If the code renderer already swapped in our json-render block, unwrap <pre>
            // so charts/tables can measure/layout normally (Recharts ResponsiveContainer
            // will often see 0px width inside a preformatted block).
            if (jsonRenderChild) {
              return <div className="not-prose">{children}</div>;
            }

            // Fallback: if this is still a raw ```json-render code fence (before our
            // code renderer swaps it), don't render inside <pre>.
            const rawJsonRenderCode = elementChildren.find((el) => {
              if (typeof el.type !== 'string' || el.type !== 'code') return false;
              const className = (el.props as { className?: unknown })?.className;
              return typeof className === 'string' && /language-json-?render\b/.test(className);
            });

            if (rawJsonRenderCode) {
              return <div className="not-prose">{children}</div>;
            }

            return <pre>{children}</pre>;
          },
          a: (props) => {
            const href = props.href;
            return (
              <a
                {...props}
                href={href}
                target={href ? '_blank' : undefined}
                rel={href ? 'noopener noreferrer' : undefined}
                onClick={async (e) => {
                  if (!href) return;

                  e.preventDefault();
                  e.stopPropagation();

                  if (isProtocolSafe(href)) {
                    await window.electron.openExternal(href);
                    return;
                  }

                  const protocol = getProtocol(href);
                  if (!protocol) return;

                  const result = await window.electron.showMessageBox({
                    type: 'question',
                    buttons: ['Cancel', 'Open'],
                    defaultId: 0,
                    title: 'Open External Link',
                    message: `Open ${protocol} link?`,
                    detail: `This will open: ${href}`,
                  });
                  if (result.response === 1) {
                    await window.electron.openExternal(href);
                  }
                }}
              />
            );
          },
          code: MarkdownCode,
        }}
      >
        {processedContent}
      </ReactMarkdown>
    </div>
  );
});

export default MarkdownContent;
