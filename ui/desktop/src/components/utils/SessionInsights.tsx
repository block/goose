import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { FolderOpen } from 'lucide-react';
import { getApiUrl, getSecretKey } from '@/config';
import { useTextAnimator } from '@/hooks/use-text-animator';
import { ActivityHeatmap } from '../sessions/ActivityHeatmap';
import { ActivityHeatmapData } from '@/types/api';

interface SessionInsights {
  totalSessions: number;
  mostActiveDirs: [string, number][];
  avgSessionDuration: number;
  totalTokens: number;
  activityHeatmap: ActivityHeatmapData[];
}

export function SessionInsights() {
  const [insights, setInsights] = useState<SessionInsights | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Add text animator effects for each number
  const totalSessionsRef = useTextAnimator({ text: insights?.totalSessions.toString() ?? '' });
  const avgDurationRef = useTextAnimator({
    text: insights?.avgSessionDuration ? insights.avgSessionDuration.toFixed(1) : '',
  });
  const totalTokensRef = useTextAnimator({
    text: insights?.totalTokens ? insights.totalTokens.toLocaleString() : '',
  });

  useEffect(() => {
    const loadInsights = async () => {
      try {
        const response = await fetch(getApiUrl('/sessions/insights'), {
          headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`Failed to fetch insights: ${response.status} ${errorText}`);
        }

        const data = await response.json();
        setInsights(data);
      } catch (error) {
        setError(error instanceof Error ? error.message : 'Failed to load insights');
      } finally {
        setLoading(false);
      }
    };

    loadInsights();
  }, []);

  if (loading) {
    return (
      <div className="grid grid-cols-2 gap-4">
        {[1, 2].map((i) => (
          <Card key={i} className="w-full">
            <CardHeader>
              <CardTitle>Loading...</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-center h-32">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (!insights) {
    return (
      <div className="grid grid-cols-2 gap-4">
        {[1, 2].map((i) => (
          <Card key={i} className="w-full">
            <CardDescription className="mb-4">No data</CardDescription>
            <CardContent>
              <div className="text-text-muted">No session data available</div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-4 gap-4">
      {/* Total Sessions Card */}
      <Card className="w-full animate-in fade-in slide-in-from-bottom-8 duration-500">
        <CardContent className="flex flex-col justify-end h-full">
          <div className="flex flex-col justify-end">
            <p
              className="text-4xl font-mono font-light h-[3rem] flex items-end"
              ref={totalSessionsRef}
            >
              {insights?.totalSessions}
            </p>
            <CardDescription>Total sessions</CardDescription>
          </div>
        </CardContent>
      </Card>

      {/* Average Duration Card */}
      <Card className="w-full animate-in fade-in slide-in-from-bottom-8 duration-500 delay-100">
        <CardContent className="flex flex-col justify-end h-full">
          <div className="flex flex-col justify-end">
            <p className="text-4xl font-mono font-light h-[3rem] flex items-end">
              <span ref={avgDurationRef}>{insights?.avgSessionDuration?.toFixed(1)}m</span>
            </p>
            <CardDescription>Avg. duration</CardDescription>
          </div>
        </CardContent>
      </Card>

      {/* Total Tokens Card */}
      <Card className="w-full col-span-2 animate-in fade-in slide-in-from-bottom-8 duration-500 delay-200">
        <CardContent className="flex flex-col justify-end h-full">
          <div className="flex flex-col justify-end">
            <p
              className="text-4xl font-mono font-light h-[3rem] flex items-end"
              ref={totalTokensRef}
            >
              {insights?.totalTokens}
            </p>
            <CardDescription>Total tokens</CardDescription>
          </div>
        </CardContent>
      </Card>

      {/* Activity Heatmap Card */}
      {/* <Card className="w-full col-span-4">
        <CardContent>
          <CardDescription className="mb-4">
            <span className="text-lg text-text-default">Activity Heatmap</span>
          </CardDescription>
          <ActivityHeatmap />
        </CardContent>
      </Card> */}

      {/* Most Active Directories Card */}
      <Card className="w-full col-span-4 animate-in fade-in slide-in-from-bottom-8 duration-500 delay-300">
        <CardContent>
          <CardDescription className="mb-4">
            <span className="text-lg text-text-default">Active directories</span>
          </CardDescription>
          <div className="space-y-2 ">
            {insights.mostActiveDirs.map(([dir, count], index) => (
              <div key={index} className="flex items-center justify-between text-sm">
                <div className="flex items-center space-x-2">
                  <FolderOpen className="h-4 w-4 text-text-muted" />
                  <span className="truncate max-w-[200px]">{dir}</span>
                </div>
                <span className="text-text-default font-mono font-light">{count} sessions</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
