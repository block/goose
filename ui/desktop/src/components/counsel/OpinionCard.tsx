import type { OpinionResponse } from '../../api/types.gen';
import MarkdownContent from '../MarkdownContent';

interface OpinionCardProps {
  opinion: OpinionResponse;
  voteCount: number;
  isWinner: boolean;
}

export function OpinionCard({ opinion, voteCount, isWinner }: OpinionCardProps) {
  return (
    <div
      className={`rounded-lg p-5 border-2 transition-all ${
        isWinner
          ? 'bg-green-500/10 border-green-500/50'
          : 'bg-background-muted border-border-subtle hover:border-border-default'
      }`}
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2">
          {isWinner && <span className="text-xl">🏆</span>}
          <h3
            className={`font-bold text-lg ${
              isWinner ? 'text-green-600 dark:text-green-400' : 'text-text-primary'
            }`}
          >
            {opinion.member_name}
          </h3>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            {Array.from({ length: voteCount }).map((_, i) => (
              <span key={i} className="text-yellow-500">
                ⭐
              </span>
            ))}
          </div>
          <span className="text-sm text-text-secondary">
            {voteCount} {voteCount === 1 ? 'vote' : 'votes'}
          </span>
        </div>
      </div>

      <div className={isWinner ? '[&_*]:!text-white' : ''}>
        <MarkdownContent content={opinion.content} />
      </div>
    </div>
  );
}
