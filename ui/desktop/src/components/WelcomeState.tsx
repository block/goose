import { Code, TestTube, FileSearch, Bug } from 'lucide-react';
import { Greeting } from './common/Greeting';
import { Goose } from './icons/Goose';

interface QuickPrompt {
  icon: React.ElementType;
  label: string;
  prompt: string;
  color: string;
}

const QUICK_PROMPTS: QuickPrompt[] = [
  {
    icon: Bug,
    label: 'Fix a bug',
    prompt: 'Help me debug and fix an issue in my code',
    color: 'text-status-error',
  },
  {
    icon: Code,
    label: 'Write code',
    prompt: 'Help me write code for a new feature',
    color: 'text-text-accent',
  },
  {
    icon: TestTube,
    label: 'Write tests',
    prompt: 'Help me write tests for my code',
    color: 'text-status-success',
  },
  {
    icon: FileSearch,
    label: 'Explain code',
    prompt: 'Help me understand how this code works',
    color: 'text-status-warning',
  },
];

interface WelcomeStateProps {
  onSubmit: (text: string) => void;
}

export function WelcomeState({ onSubmit }: WelcomeStateProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full px-6 py-8 max-w-3xl mx-auto">
      <div className="flex flex-col items-center mb-8">
        <div className="w-16 h-16 mb-4 opacity-80">
          <Goose />
        </div>
        <Greeting />
        <p className="text-text-muted text-sm mt-2">What would you like to work on?</p>
      </div>

      <div className="grid grid-cols-2 gap-3 w-full">
        {QUICK_PROMPTS.map((qp) => (
          <button
            key={qp.label}
            onClick={() => onSubmit(qp.prompt)}
            className="flex items-center gap-3 px-4 py-3 rounded-xl
              border border-border-muted bg-background-default
              hover:bg-background-muted hover:border-border-default
              transition-all duration-200 text-left group"
          >
            <qp.icon className={`w-5 h-5 ${qp.color} flex-shrink-0`} />
            <div className="min-w-0">
              <div className="text-sm font-medium text-text-default">
                {qp.label}
              </div>
              <div className="text-xs text-text-subtle truncate">{qp.prompt}</div>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
