import type * as React from 'react';
import { cn } from '../../../utils';

interface CodeBlockProps extends React.ComponentProps<'div'> {
  code: string;
  language?: string;
}

export function CodeBlock({ code, language, className, ...props }: CodeBlockProps) {
  return (
    <div
      className={cn(
        'rounded-lg border border-border-default bg-background-muted overflow-hidden',
        className
      )}
      {...props}
    >
      {language && (
        <div className="flex items-center justify-between px-4 py-1.5 bg-background-active border-b border-border-default">
          <span className="text-xs text-text-muted font-mono">{language}</span>
        </div>
      )}
      <pre className="p-4 overflow-x-auto text-sm">
        <code className="font-mono text-text-default">{code}</code>
      </pre>
    </div>
  );
}
