import type { OpinionResponse } from '../../api/types.gen';

interface VotingChartProps {
  opinions: OpinionResponse[];
  voteCounts: Record<string, number>;
}

export function VotingChart({ opinions, voteCounts }: VotingChartProps) {
  const maxVotes = Math.max(...Object.values(voteCounts), 1);

  const sortedOpinions = [...opinions].sort((a, b) => {
    const votesA = voteCounts[a.member_id] || 0;
    const votesB = voteCounts[b.member_id] || 0;
    return votesB - votesA;
  });

  return (
    <div className="bg-background-muted rounded-lg p-5 border border-border-subtle">
      <h3 className="text-lg font-bold text-text-primary mb-4">📊 Voting Results</h3>
      <div className="space-y-3">
        {sortedOpinions.map((opinion) => {
          const votes = voteCounts[opinion.member_id] || 0;
          const percentage = maxVotes > 0 ? (votes / maxVotes) * 100 : 0;

          return (
            <div key={opinion.member_id} className="space-y-1">
              <div className="flex items-center justify-between text-sm">
                <span className="font-medium text-text-primary">{opinion.member_name}</span>
                <span className="text-text-secondary">
                  {votes} {votes === 1 ? 'vote' : 'votes'}
                </span>
              </div>
              <div className="h-2 bg-background-subtle rounded-full overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-accent-primary to-accent-primary-hover transition-all duration-500 ease-out rounded-full"
                  style={{ width: `${percentage}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
