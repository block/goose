import React from "react";
import { type MarkdownWidget, type ColorTheme } from "../types";
import ReactMarkdown from "react-markdown";
import { cn } from "@/lib/utils";
import "./markdown.css";

interface MarkdownWidgetProps {
  widget: MarkdownWidget;
  data?: any;
}

const MarkdownWidget: React.FC<MarkdownWidgetProps> = ({ widget }) => {
  const colorTheme = widget.properties.color || '#0ea5e9' as ColorTheme; // Default to blue if no theme selected

  return (
    <div 
      className={cn(
        "w-full h-full overflow-auto prose dark:prose-invert max-w-none",
        "prose-headings:font-semibold",
        "prose-h1:text-3xl prose-h2:text-2xl prose-h3:text-xl",
        "prose-a:no-underline hover:prose-a:underline",
        "prose-code:rounded prose-code:px-1",
        "prose-blockquote:border-l-4 prose-blockquote:pl-4",
        "prose-img:rounded-lg"
      )}
      style={{
        '--theme-color': colorTheme,
        '--theme-color-light': colorTheme.startsWith('#') 
          ? `${colorTheme}33` 
          : colorTheme.replace(')', ', 0.2)'),
        '--theme-color-medium': colorTheme.startsWith('#')
          ? `${colorTheme}66`
          : colorTheme.replace(')', ', 0.4)'),
      } as React.CSSProperties}
    >
      <ReactMarkdown>{widget.config?.content || ''}</ReactMarkdown>
    </div>
  );
};

export default MarkdownWidget; 