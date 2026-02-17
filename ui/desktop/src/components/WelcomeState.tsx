import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  MessageSquare,
  Sparkles,
  Zap,
  Clock,
  ArrowRight,
  Code,
  TestTube,
  FileSearch,
  Bug,
} from 'lucide-react';
import { getSessionInsights, listSessions, Session, SessionInsights } from '../api';
import { resumeSession } from '../sessions';
import { useNavigation } from '../hooks/useNavigation';
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
  const [insights, setInsights] = useState<SessionInsights | null>(null);
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const navigate = useNavigate();
  const setView = useNavigation();

  useEffect(() => {
    const loadData = async () => {
      try {
        const [insightsRes, sessionsRes] = await Promise.allSettled([
          getSessionInsights({ throwOnError: true }),
          listSessions<true>({ throwOnError: true }),
        ]);

        if (insightsRes.status === 'fulfilled') {
          setInsights(insightsRes.value.data);
        }
        if (sessionsRes.status === 'fulfilled') {
          setRecentSessions(sessionsRes.value.data.sessions.slice(0, 5));
        }
      } finally {
        setIsLoading(false);
      }
    };

    const timeout = setTimeout(() => setIsLoading(false), 5000);
    loadData();
    return () => clearTimeout(timeout);
  }, []);

  const handleResumeSession = (session: Session) => {
    resumeSession(session, setView);
  };

  const formatTimeAgo = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return `${days}d ago`;
  };

  return (
    <div className="flex flex-col items-center justify-center h-full px-6 py-8 max-w-3xl mx-auto">
      {/* Greeting + Logo */}
      <div className="flex flex-col items-center mb-8">
        <div className="w-16 h-16 mb-4 opacity-80">
          <Goose />
        </div>
        <Greeting />
        <p className="text-text-muted text-sm mt-2">What would you like to work on?</p>
      </div>

      {/* Quick Start Prompts */}
      <div className="grid grid-cols-2 gap-3 w-full mb-8">
        {QUICK_PROMPTS.map((qp) => (
          <button
            key={qp.label}
            onClick={() => onSubmit(qp.prompt)}
            className="flex items-center gap-3 px-4 py-3 rounded-xl border border-border-muted bg-background-default hover:bg-background-muted hover:border-border-default transition-all duration-200 text-left group"
          >
            <qp.icon className={`w-5 h-5 ${qp.color} flex-shrink-0`} />
            <div className="min-w-0">
              <div className="text-sm font-medium text-text-default group-hover:text-text-default">
                {qp.label}
              </div>
              <div className="text-xs text-text-subtle truncate">{qp.prompt}</div>
            </div>
          </button>
        ))}
      </div>

      {/* Compact Stats Row */}
      {!isLoading && insights && (
        <div className="flex items-center gap-6 mb-8 text-xs text-text-muted">
          <div className="flex items-center gap-1.5">
            <MessageSquare className="w-3.5 h-3.5" />
            <span>
              {insights.totalSessions} {insights.totalSessions === 1 ? 'session' : 'sessions'}
            </span>
          </div>
          <div className="flex items-center gap-1.5">
            <Zap className="w-3.5 h-3.5" />
            <span>{(insights.totalTokens ?? 0).toLocaleString()} tokens</span>
          </div>
          <div className="flex items-center gap-1.5">
            <Sparkles className="w-3.5 h-3.5" />
            <span>Ready to help</span>
          </div>
        </div>
      )}

      {/* Recent Sessions */}
      {recentSessions.length > 0 && (
        <div className="w-full">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
              Recent Sessions
            </h3>
            <button
              onClick={() => navigate('/sessions')}
              className="text-xs text-text-accent hover:underline flex items-center gap-1"
            >
              View all <ArrowRight className="w-3 h-3" />
            </button>
          </div>
          <div className="space-y-1">
            {recentSessions.map((session) => (
              <button
                key={session.id}
                onClick={() => handleResumeSession(session)}
                className="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-background-muted transition-colors text-left group"
              >
                <MessageSquare className="w-4 h-4 text-text-subtle flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-text-default truncate">
                    {session.name || 'Untitled session'}
                  </div>
                </div>
                <div className="flex items-center gap-1 text-xs text-text-subtle">
                  <Clock className="w-3 h-3" />
                  <span>{formatTimeAgo(session.updated_at)}</span>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
