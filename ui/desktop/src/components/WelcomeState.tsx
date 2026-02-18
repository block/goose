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
import { Greeting } from './common/Greeting';
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
    prompt: 'Help me write code for a new feature',
    color: 'text-text-accent',
  },
  {
    icon: Bug,
    label: 'Debug & fix issues',
    description: 'Trace bugs, read stack traces, and apply targeted fixes',
    prompt: 'Help me debug and fix an issue in my code',
    color: 'text-status-error',
  },
  {
    icon: TestTube,
    label: 'Write & run tests',
    description: 'Generate unit, integration, and e2e tests with coverage',
    prompt: 'Help me write comprehensive tests for my project',
    color: 'text-status-success',
  },
  {
    icon: FileSearch,
    label: 'Understand a codebase',
    description: 'Navigate, explain, and document unfamiliar code',
    prompt: 'Help me understand how this codebase is structured',
    color: 'text-status-warning',
  },
  {
    icon: Terminal,
    label: 'Run shell commands',
    description: 'Execute builds, scripts, git operations, and CLI tools',
    prompt: 'Help me set up and run my development environment',
    color: 'text-text-muted',
  },
  {
    icon: GitBranch,
    label: 'Manage git workflow',
    description: 'Branch, commit, rebase, resolve conflicts, open PRs',
    prompt: 'Help me manage my git branches and prepare a clean PR',
    color: 'text-status-info',
  },
  {
    icon: Server,
    label: 'DevOps & infrastructure',
    description: 'Docker, CI/CD pipelines, deployment configs',
    prompt: 'Help me set up CI/CD and deployment for my project',
    color: 'text-text-subtle',
  },
  {
    icon: ShieldCheck,
    label: 'Security & quality audit',
    description: 'Find vulnerabilities, lint issues, and code smells',
    prompt: 'Run a security and quality audit on my codebase',
    color: 'text-status-error',
  },
  {
    icon: Pencil,
    label: 'Write documentation',
    description: 'READMEs, API docs, architecture guides, changelogs',
    prompt: 'Help me write documentation for my project',
    color: 'text-text-accent',
  },
  {
    icon: LayoutDashboard,
    label: 'Build UI components',
    description: 'React, Next.js, Tailwind — from mockup to pixel-perfect',
    prompt: 'Help me build a responsive UI component',
    color: 'text-status-success',
  },
  {
    icon: Globe,
    label: 'Research & learn',
    description: 'Explore APIs, compare libraries, learn new tech',
    prompt: 'Help me research the best approach for my use case',
    color: 'text-status-warning',
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
  return (
    <div className="flex flex-col items-center px-8 py-12 max-w-3xl mx-auto">
      {/* Header */}
      <div className="flex flex-col items-center mb-12">
        <div className="w-16 h-16 mb-5">
          <Goose />
        </div>
        <Greeting />
        <h2 className="text-2xl font-semibold text-text-default mt-5 tracking-tight">
          What can be achieved?
        </h2>
        <p className="text-text-muted text-sm mt-2 text-center max-w-lg leading-relaxed">
          Goose is your AI-powered development partner. Pick a starting point or just type what you need.
        </p>
      </div>

      {/* Capability grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 w-full mb-10">
        {CAPABILITIES.map((cap) => (
          <button
            key={cap.label}
            onClick={() => {
              if (cap.prompt) {
                onSubmit(cap.prompt);
              }
            }}
            disabled={!cap.prompt}
            className="flex flex-col items-start gap-2.5 px-5 py-4 rounded-xl
              border border-border-muted bg-background-default
              hover:bg-background-muted hover:border-border-default
              hover:shadow-md
              active:scale-[0.98]
              transition-all duration-200 text-left group
              disabled:opacity-50 disabled:cursor-default disabled:hover:shadow-none"
          >
            <cap.icon
              className={`w-5 h-5 ${cap.color} flex-shrink-0
                group-hover:scale-110 transition-transform duration-200`}
            />
            <div className="min-w-0 w-full">
              <div className="text-sm font-medium text-text-default leading-tight">
                {cap.label}
              </div>
              <div className="text-sm text-text-muted leading-snug mt-1">
                {cap.description}
              </div>
            </div>
          </button>
        ))}
      </div>

      {/* Footer hint */}
      <p className="text-text-muted text-xs text-center">
        Type{' '}
        <kbd className="px-1.5 py-0.5 rounded border border-border-muted bg-background-muted text-text-default font-mono text-[11px]">
          /
        </kbd>{' '}
        for slash commands · Drag files to share them
      </p>
    </div>
  );
}
