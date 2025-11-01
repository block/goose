import type { OpinionResponse } from '../../api/types.gen';
import MarkdownContent from '../MarkdownContent';

interface WinnerDisplayProps {
  winner: OpinionResponse;
  voteCount: number;
  totalVotes: number;
}

export function WinnerDisplay({ winner, voteCount, totalVotes }: WinnerDisplayProps) {
  return (
    <div className="bg-gradient-to-br from-green-500/10 to-green-600/5 border-2 border-green-500/50 rounded-xl p-6">
      <div className="flex items-start gap-4">
        <div className="text-4xl">🏆</div>
        <div className="flex-1 space-y-3">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <h3 className="text-2xl font-bold text-green-600 dark:text-green-400">
                {winner.member_name}
              </h3>
              <span className="px-3 py-1 bg-green-500/20 text-green-600 dark:text-green-400 text-sm font-medium rounded-full">
                Winner
              </span>
            </div>
            <div className="flex items-center gap-2 text-sm text-white">
              <span className="flex items-center gap-1">
                {Array.from({ length: voteCount }).map((_, i) => (
                  <span key={i} className="text-yellow-500">
                    ⭐
                  </span>
                ))}
              </span>
              <span>
                {voteCount} out of {totalVotes} votes
              </span>
            </div>
          </div>

          <div className="[&_*]:!text-white">
            <MarkdownContent content={winner.content} />
          </div>
        </div>
      </div>
    </div>
  );
}
