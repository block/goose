import {
  Bug,
  Code,
  FileSearch,
  GitBranch,
  Globe,
  LayoutDashboard,
  MessageSquare,
  Pencil,
  Send,
  Server,
  ShieldCheck,
  Terminal,
  TestTube,
} from 'lucide-react';
import React from 'react';

import { Goose } from './icons/Goose';

interface Capability {
  icon: React.ElementType;
  label: string;
  description: string;
  prompt: string;
  color: string;
}

const CAPABILITIES: Capability[] = [
  {
    icon: Code,
    label: 'Write & refactor code',
    description: 'Generate, transform, or modernize code across languages',
    prompt: 'Help me write clean, well-structured code',
    color: 'text-accent',
  },
  {
    icon: Bug,
    label: 'Debug & fix issues',
    description: 'Trace bugs, read stack traces, and apply targeted fixes',
    prompt: 'Help me debug and fix an issue in my code',
    color: 'text-text-danger',
  },
  {
    icon: TestTube,
    label: 'Write & run tests',
    description: 'Generate unit, integration, and e2e tests with coverage',
    prompt: 'Help me write comprehensive tests for my code',
    color: 'text-text-success',
  },
  {
    icon: FileSearch,
    label: 'Understand a codebase',
    description: 'Navigate, explain, and document unfamiliar code',
    prompt: 'Help me understand this codebase and how it works',
    color: 'text-text-warning',
  },
  {
    icon: Terminal,
    label: 'Run shell commands',
    description: 'Execute builds, scripts, git operations, and CLI tools',
    prompt: 'Help me with shell commands and scripting',
    color: 'text-text-info',
  },
  {
    icon: GitBranch,
    label: 'Manage git workflow',
    description: 'Branch, commit, rebase, resolve conflicts, open PRs',
    prompt: 'Help me manage my git workflow',
    color: 'text-text-muted',
  },
  {
    icon: Server,
    label: 'DevOps & infrastructure',
    description: 'Docker, CI/CD pipelines, deployment configs',
    prompt: 'Help me with DevOps and infrastructure setup',
    color: 'text-text-subtle',
  },
  {
    icon: ShieldCheck,
    label: 'Security & quality audit',
    description: 'Find vulnerabilities, lint issues, and code smells',
    prompt: 'Audit my code for security issues and quality problems',
    color: 'text-text-danger',
  },
  {
    icon: Pencil,
    label: 'Write documentation',
    description: 'READMEs, API docs, architecture guides, changelogs',
    prompt: 'Help me write clear documentation for my project',
    color: 'text-text-success',
  },
  {
    icon: LayoutDashboard,
    label: 'Build UI components',
    description: 'React, Next.js, Tailwind — from mockup to pixel-perfect',
    prompt: 'Help me build a UI component',
    color: 'text-accent',
  },
  {
    icon: Globe,
    label: 'Research & learn',
    description: 'Explore APIs, compare libraries, learn new tech',
    prompt: 'Help me research and learn about a technology',
    color: 'text-text-info',
  },
  {
    icon: MessageSquare,
    label: 'Anything else',
    description: 'Just describe what you need — Goose adapts',
    prompt: '',
    color: 'text-text-muted',
  },
];

interface WelcomeStateProps {
  onSubmit: (text: string) => void;
}

export function WelcomeState({ onSubmit }: WelcomeStateProps) {
  const [input, setInput] = React.useState('');
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  const handleSubmit = React.useCallback(() => {
    const text = input.trim();
    if (!text) return;
    onSubmit(text);
    setInput('');
  }, [input, onSubmit]);

  const handleKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit]
  );

  return (
    <div className="flex flex-col items-center px-8 py-12 max-w-3xl mx-auto">
      {/* Vertical space + Goose icon */}
      <div className="pt-8" />
      <Goose className="size-12" />
      <div className="pt-8" />

      {/* Big title */}
      <h1 className="text-3xl font-bold tracking-tight text-text-default text-center">
        Ready to create something great?
      </h1>

      {/* Subtitle */}
      <h2 className="text-lg font-medium text-text-muted mt-2 text-center">
        What can be achieved?
      </h2>

      {/* Paragraph */}
      <p className="text-sm text-text-subtle mt-3 text-center max-w-lg leading-relaxed">
        Goose is your AI-powered development partner. Pick a starting point or just type what you
        need.
      </p>

      {/* Small vertical space + grid */}
      <div className="pt-8" />

      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 w-full">
        {CAPABILITIES.map((cap) => (
          <button
            key={cap.label}
            onClick={() => cap.prompt && onSubmit(cap.prompt)}
            disabled={!cap.prompt}
            className="group flex flex-col items-start gap-2 px-4 py-3.5 rounded-xl
              border border-border-default bg-background-default
              hover:bg-background-muted hover:border-border-strong hover:shadow-md
              active:scale-[0.98]
              disabled:opacity-40 disabled:cursor-default disabled:hover:bg-background-default disabled:hover:border-border-default disabled:hover:shadow-none
              transition-all duration-150 text-left cursor-pointer"
          >
            <cap.icon
              className={`size-5 ${cap.color} transition-transform group-hover:scale-110`}
            />
            <div>
              <div className="text-sm font-medium text-text-default">{cap.label}</div>
              <div className="text-sm text-text-muted leading-snug mt-0.5">{cap.description}</div>
            </div>
          </button>
        ))}
      </div>

      {/* Inline input — matches ChatInput styling */}
      <div className="w-full mt-8">
        <div
          className="relative flex items-end bg-background-default border border-border-default rounded-t-2xl
            hover:border-border-strong focus-within:border-border-strong
            transition-all p-4 z-10"
        >
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="What should goose do?"
            rows={1}
            className="w-full outline-none border-none focus:ring-0 bg-transparent px-3 pt-1 pb-0.5 text-sm
              resize-none text-text-default placeholder:text-text-muted overflow-y-auto"
            style={{ maxHeight: '150px' }}
          />
          <button
            onClick={handleSubmit}
            disabled={!input.trim()}
            className="ml-2 p-1.5 rounded-lg transition-all
              disabled:opacity-30 disabled:cursor-not-allowed
              text-text-muted hover:text-text-default hover:bg-background-muted"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
