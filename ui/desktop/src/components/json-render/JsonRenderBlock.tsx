import { memo, useMemo, useState } from 'react';
import { CatalogRenderer } from './setup';

interface JsonRenderBlockProps {
  spec: string;
}

export const JsonRenderBlock = memo(function JsonRenderBlock({ spec }: JsonRenderBlockProps) {
  const [error, setError] = useState<string | null>(null);

  const parsedSpec = useMemo(() => {
    try {
      setError(null);
      return JSON.parse(spec);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Invalid JSON');
      return null;
    }
  }, [spec]);

  if (error) {
    return (
      <div className="p-4 rounded-lg border border-red-300 bg-red-50 dark:border-red-800 dark:bg-red-950">
        <p className="text-sm text-red-600 dark:text-red-400">
          Failed to render component: {error}
        </p>
      </div>
    );
  }

  if (!parsedSpec) return null;

  return (
    <div className="my-2 rounded-lg border border-border-default p-4 bg-background-default">
      <CatalogRenderer spec={parsedSpec} />
    </div>
  );
});
