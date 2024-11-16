import React, { useState } from 'react';
import { motion } from "framer-motion";
import { BoxIcon, GPSIcon } from "./ui/icons";
import ReactMarkdown from 'react-markdown';

interface ToolCallProps {
  call: {
    state: 'call' | 'result'
    toolCallId: string
    toolName: string
    args: Record<string, any>
  }
}

const getColorFromState = ({
  state,
  type,
}: {
  state: string;
  type: "foreground" | "text";
}) => {
  switch (state) {
    case "running":
      return type === "foreground" ? "#3b82f6" : "#eff6ff";
    case "completed":
      return type === "foreground" ? "#10b981" : "#f0fdf4";
    default:
      return type === "foreground" ? "#f4f4f5" : "#71717a";
  }
};

const convertArgsToMarkdown = (args: Record<string, any>): string => {
  const lines: string[] = [];
  
  Object.entries(args).forEach(([key, value]) => {
    // Add the parameter name as a heading
    lines.push(`### ${key}`);
    lines.push('');
    
    // Handle different value types
    if (typeof value === 'string') {
      lines.push('```');
      lines.push(value);
      lines.push('```');
    } else if (Array.isArray(value)) {
      value.forEach((item, index) => {
        lines.push(`${index + 1}. ${JSON.stringify(item)}`);
      });
    } else if (typeof value === 'object' && value !== null) {
      lines.push('```json');
      lines.push(JSON.stringify(value, null, 2));
      lines.push('```');
    } else {
      lines.push('```');
      lines.push(String(value));
      lines.push('```');
    }
    lines.push('');
  });
  
  return lines.join('\n');
};

export default function ToolCall({ call }: ToolCallProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  if (!call) return null;

  const markdownContent = convertArgsToMarkdown(call.args);

  if (call.state === 'result') {
    return (
      <div className="my-4 flex flex-col gap-2 w-full">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex flex-row items-center gap-2 text-sm text-zinc-400 hover:text-zinc-300 transition-colors"
        >
          <BoxIcon size={14} />
          <span>Tool Result</span>
          <span className="text-zinc-500">{call.toolCallId}</span>
          <span className="ml-2">{isExpanded ? '▼' : '▶'}</span>
        </button>
        
        {isExpanded && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2 }}
            className="mt-2 font-mono text-sm text-green-400 bg-black bg-opacity-50 rounded-md p-3 overflow-x-auto"
          >
            <ReactMarkdown
              components={{
                code({ node, inline, className, children, ...props }) {
                  return (
                    <code className={`${className} ${inline ? 'bg-black bg-opacity-25 px-1 py-0.5 rounded' : ''}`} {...props}>
                      {children}
                    </code>
                  )
                }
              }}
            >
              {markdownContent}
            </ReactMarkdown>
          </motion.div>
        )}
      </div>
    );
  }

  return (
    <div className="my-4 flex flex-col gap-4 w-full">
      <motion.div
        className="flex flex-col justify-between items-start"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.1 }}
      >
        <div className="flex flex-row gap-2 items-center text-sm text-zinc-400">
          <BoxIcon size={14} />
          <div>Tool Call</div>
          <div className="text-zinc-500">{call.toolCallId}</div>
        </div>
      </motion.div>

      <motion.div
        className="flex flex-row items-center gap-3"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.2 }}
      >
        <motion.div
          className="size-8 rounded-full flex-shrink-0 flex flex-row justify-center items-center"
          initial={{ background: "#3b82f6" }}
          animate={{
            background: getColorFromState({
              state: "running",
              type: "foreground",
            }),
            color: getColorFromState({
              state: "running",
              type: "text",
            }),
          }}
        >
          <BoxIcon size={14} />
        </motion.div>

        <div className="h-2 bg-zinc-100 dark:bg-zinc-700 w-24 rounded-lg relative">
          <motion.div
            className="h-2 rounded-lg z-10 absolute"
            initial={{ width: 0, background: "#3b82f6" }}
            animate={{
              width: "100%",
              background: getColorFromState({
                state: "running",
                type: "foreground",
              }),
            }}
            transition={{ delay: 0.3 }}
          />
        </div>

        <div className="flex-grow">
          <div className="font-mono text-sm text-white">
            {call.toolName}
          </div>
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            transition={{ delay: 0.3 }}
            className="mt-2 font-mono text-sm text-green-400 bg-black bg-opacity-50 rounded-md p-3 overflow-x-auto"
          >
            <ReactMarkdown
              components={{
                code({ node, inline, className, children, ...props }) {
                  return (
                    <code className={`${className} ${inline ? 'bg-black bg-opacity-25 px-1 py-0.5 rounded' : ''}`} {...props}>
                      {children}
                    </code>
                  )
                }
              }}
            >
             {markdownContent}
            </ReactMarkdown>
          </motion.div>
        </div>

        <div className="h-2 bg-zinc-100 dark:bg-zinc-700 w-24 rounded-lg relative">
          <motion.div
            className="h-2 rounded-lg z-10 absolute"
            initial={{ width: 0, background: "#f4f4f5" }}
            animate={{
              width: "100%",
              background: getColorFromState({
                state: "pending",
                type: "foreground",
              }),
            }}
            transition={{ delay: 0.4 }}
          />
        </div>

        <motion.div
          className="size-8 rounded-full flex-shrink-0 flex flex-row justify-center items-center"
          initial={{ background: "#f4f4f5" }}
          animate={{
            background: getColorFromState({
              state: "pending",
              type: "foreground",
            }),
            color: getColorFromState({
              state: "pending",
              type: "text",
            }),
          }}
        >
          <GPSIcon size={14} />
        </motion.div>

        <div className="h-2 bg-zinc-100 dark:bg-zinc-700 w-24 rounded-lg relative">
          <motion.div
            className="h-2 rounded-lg z-10 absolute"
            initial={{ width: 0, background: "#f4f4f5" }}
            animate={{
              width: "100%",
              background: getColorFromState({
                state: "pending",
                type: "foreground",
              }),
            }}
            transition={{ delay: 0.5 }}
          />
        </div>

        <motion.div
          className="size-8 rounded-full flex-shrink-0 flex flex-row justify-center items-center"
          initial={{ background: "#f4f4f5" }}
          animate={{
            background: getColorFromState({
              state: "pending",
              type: "foreground",
            }),
            color: getColorFromState({
              state: "pending",
              type: "text",
            }),
          }}
        >
          <BoxIcon size={14} />
        </motion.div>
      </motion.div>
    </div>
  );
}