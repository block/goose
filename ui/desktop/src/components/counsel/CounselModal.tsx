import { useState } from 'react';
import { counsel } from '../../api';
import type { CounselResponse } from '../../api/types.gen';
import { OpinionCard } from './OpinionCard';
import { VotingChart } from './VotingChart';
import { WinnerDisplay } from './WinnerDisplay';
import GooseLogo from '../GooseLogo';

interface CounselModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function CounselModal({ isOpen, onClose }: CounselModalProps) {
  const [prompt, setPrompt] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<CounselResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showAllOpinions, setShowAllOpinions] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!prompt.trim()) return;

    setIsLoading(true);
    setError(null);
    setResult(null);

    try {
      const response = await counsel({
        body: {
          prompt: prompt.trim(),
        },
      });

      if (response.data) {
        setResult(response.data);
      } else if (response.error) {
        const errorMessage =
          typeof response.error === 'object' && 'message' in response.error
            ? String(response.error.message)
            : 'Failed to get counsel';
        setError(errorMessage);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An unexpected error occurred');
    } finally {
      setIsLoading(false);
    }
  };

  const handleReset = () => {
    setPrompt('');
    setResult(null);
    setError(null);
    setShowAllOpinions(false);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-background-default rounded-xl shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col m-4">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-border-subtle">
          <div className="flex items-center gap-3">
            <span className="text-3xl">🎭</span>
            <div>
              <h2 className="text-2xl font-bold text-text-primary">Counsel of 9</h2>
              <p className="text-sm text-text-secondary">
                Get diverse perspectives from nine AI personas
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-text-secondary hover:text-text-primary transition-colors p-2"
            aria-label="Close"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {!result && !isLoading && (
            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label
                  htmlFor="counsel-prompt"
                  className="block text-sm font-medium text-text-primary mb-2"
                >
                  What would you like counsel on?
                </label>
                <textarea
                  id="counsel-prompt"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="e.g., Should I use microservices or a monolithic architecture?"
                  className="w-full h-32 px-4 py-3 bg-background-muted border border-border-subtle rounded-lg text-text-primary placeholder-text-tertiary focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent resize-none"
                  disabled={isLoading}
                />
              </div>

              <div className="flex items-center gap-4">
                <button
                  type="submit"
                  disabled={isLoading || !prompt.trim()}
                  className="px-6 py-3 bg-accent-primary hover:bg-accent-primary-hover disabled:bg-background-muted disabled:text-text-tertiary text-white font-medium rounded-lg transition-colors"
                >
                  Ask Counsel
                </button>
                <p className="text-sm text-text-secondary">
                  This will take 30-60 seconds as we gather opinions from all 9 personas
                </p>
              </div>
            </form>
          )}

          {isLoading && (
            <div className="flex flex-col items-center justify-center py-12 space-y-6">
              <GooseLogo size="default" hover={false} className="animate-pulse" />
              <div className="text-center space-y-2">
                <p className="text-lg font-medium text-text-primary">
                  Deliberating with the Counsel of 9...
                </p>
                <p className="text-sm text-text-secondary">
                  Phase 1: Gathering opinions from 9 personas
                </p>
                <p className="text-sm text-text-secondary">Phase 2: Conducting voting</p>
              </div>
            </div>
          )}

          {error && (
            <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <svg
                  className="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                    clipRule="evenodd"
                  />
                </svg>
                <div className="flex-1">
                  <p className="font-medium text-red-500">Error</p>
                  <p className="text-sm text-text-secondary mt-1">{error}</p>
                </div>
              </div>
              <button
                onClick={handleReset}
                className="mt-4 px-4 py-2 bg-red-500/20 hover:bg-red-500/30 text-red-500 rounded-lg transition-colors"
              >
                Try Again
              </button>
            </div>
          )}

          {result && (
            <div className="space-y-6">
              {/* Winner Display */}
              <WinnerDisplay
                winner={result.winner}
                voteCount={result.vote_counts[result.winner.member_id] || 0}
                totalVotes={result.total_votes}
              />

              {/* Voting Chart */}
              <VotingChart opinions={result.all_opinions} voteCounts={result.vote_counts} />

              {/* Toggle All Opinions */}
              <div className="flex items-center justify-between py-4 border-t border-border-subtle">
                <button
                  onClick={() => setShowAllOpinions(!showAllOpinions)}
                  className="flex items-center gap-2 text-accent-primary hover:text-accent-primary-hover font-medium transition-colors"
                >
                  <span>{showAllOpinions ? 'Hide' : 'Show'} All Opinions</span>
                  <svg
                    className={`w-5 h-5 transition-transform ${showAllOpinions ? 'rotate-180' : ''}`}
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M19 9l-7 7-7-7"
                    />
                  </svg>
                </button>
                <span className="text-sm text-text-secondary">
                  {result.all_opinions.length}/9 members participated
                </span>
              </div>

              {/* All Opinions */}
              {showAllOpinions && (
                <div className="space-y-4">
                  {result.all_opinions
                    .sort((a, b) => {
                      const votesA = result.vote_counts[a.member_id] || 0;
                      const votesB = result.vote_counts[b.member_id] || 0;
                      return votesB - votesA;
                    })
                    .map((opinion) => (
                      <OpinionCard
                        key={opinion.member_id}
                        opinion={opinion}
                        voteCount={result.vote_counts[opinion.member_id] || 0}
                        isWinner={opinion.member_id === result.winner.member_id}
                      />
                    ))}
                </div>
              )}

              {/* Unavailable Members */}
              {result.unavailable_members.length > 0 && (
                <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-lg p-4">
                  <p className="text-sm font-medium text-yellow-600 dark:text-yellow-500">
                    ⚠️ Unavailable members: {result.unavailable_members.join(', ')}
                  </p>
                </div>
              )}

              {/* Actions */}
              <div className="flex items-center gap-3 pt-4 border-t border-border-subtle">
                <button
                  onClick={handleReset}
                  className="px-6 py-3 bg-accent-primary hover:bg-accent-primary-hover text-white font-medium rounded-lg transition-colors"
                >
                  New Question
                </button>
                <button
                  onClick={onClose}
                  className="px-6 py-3 bg-background-muted hover:bg-background-subtle text-text-primary font-medium rounded-lg transition-colors"
                >
                  Close
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
