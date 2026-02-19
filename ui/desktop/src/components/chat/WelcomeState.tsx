import {
  Bug,
  Code,
  FileSearch,
  GitBranch,
  Globe,
  LayoutDashboard,
  MessageSquare,
  Pencil,
  Server,
  ShieldCheck,
  Terminal,
  TestTube,
} from 'lucide-react';
import type React from 'react';

import { Goose } from '../icons/Goose';

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
    prompt: 'Help me write or refactor some code',
    color: 'text-accent',
  },
  {
    icon: Bug,
    label: 'Debug & fix issues',
    description: 'Trace bugs, read stack traces, and apply targeted fixes',
    prompt: 'Help me debug and fix an issue',
    color: 'text-text-danger',
  },
  {
    icon: TestTube,
    label: 'Write & run tests',
    description: 'Generate unit, integration, and e2e tests with coverage',
    prompt: 'Help me write tests for my code',
    color: 'text-text-success',
  },
  {
    icon: FileSearch,
    label: 'Understand a codebase',
    description: 'Navigate, explain, and document unfamiliar code',
    prompt: 'Help me understand this codebase',
    color: 'text-text-warning',
  },
  {
    icon: Terminal,
    label: 'Run shell commands',
    description: 'Execute builds, scripts, git operations, and CLI tools',
    prompt: 'Help me run some shell commands',
    color: 'text-text-info',
  },
  {
    icon: GitBranch,
    label: 'Manage git workflow',
    description: 'Branch, commit, rebase, resolve conflicts, open PRs',
    prompt: 'Help me with my git workflow',
    color: 'text-text-muted',
  },
  {
    icon: Server,
    label: 'DevOps & infrastructure',
    description: 'Docker, CI/CD pipelines, deployment configs',
    prompt: 'Help me with DevOps and infrastructure',
    color: 'text-text-subtle',
  },
  {
    icon: ShieldCheck,
    label: 'Security & quality audit',
    description: 'Find vulnerabilities, lint issues, and code smells',
    prompt: 'Run a security and quality audit on my code',
    color: 'text-text-danger',
  },
  {
    icon: Pencil,
    label: 'Write documentation',
    description: 'READMEs, API docs, architecture guides, changelogs',
    prompt: 'Help me write documentation',
    color: 'text-text-success',
  },
  {
    icon: LayoutDashboard,
    label: 'Build UI components',
    description: 'React, Next.js, Tailwind — from mockup to pixel-perfect',
    prompt: 'Help me build a UI component',
    color: 'text-text-info',
  },
  {
    icon: Globe,
    label: 'Research & learn',
    description: 'Explore APIs, compare libraries, learn new tech',
    prompt: 'Help me research and learn about a topic',
    color: 'text-text-warning',
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

export default function WelcomeState({ onSubmit }: WelcomeStateProps) {
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

      {/* Footer hint */}
      <p className="text-xs text-text-subtle mt-8 text-center">
        Type{' '}
        <kbd className="px-1.5 py-0.5 rounded bg-background-muted text-[11px] text-text-default font-mono">
          /
        </kbd>{' '}
        for slash commands · Drag files into the chat
      </p>
    </div>
  );
}
