import React from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeRaw from 'rehype-raw';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { visit } from 'unist-util-visit';
import CopyButton from './ui/CopyButton';

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
  return (
    <div className="relative group w-full">
      <CopyButton
        text={children}
        className="absolute right-2 bottom-2 p-1.5 rounded-lg bg-gray-700/50 text-gray-300
                 opacity-0 group-hover:opacity-100 transition-opacity duration-200
                 hover:bg-gray-600/50 hover:text-gray-100 z-10"
      />
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
  return (
    <div className="w-full overflow-x-hidden">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeinlineCodeProperty, rehypeRaw]}
        className={`prose prose-xs dark:prose-invert w-full max-w-full break-words
          prose-pre:p-0 prose-pre:m-0 !p-0
          prose-code:break-all prose-code:whitespace-pre-wrap
          prose-table:table prose-table:w-full
          prose-td:border prose-td:border-borderSubtle prose-td:p-2
          prose-th:border prose-th:border-borderSubtle prose-th:p-2
          prose-thead:bg-bgSubtle
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
