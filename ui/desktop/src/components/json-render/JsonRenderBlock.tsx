import { useMemo } from 'react';
import { nestedToFlat } from '@json-render/core';
import { CatalogRenderer } from './setup';

interface JsonRenderBlockProps {
  spec: string;
}

export const JsonRenderBlock = ({ spec }: JsonRenderBlockProps) => {
  const { parsedSpec, error } = useMemo(() => {
    try {
      const raw = JSON.parse(spec);
      // The LLM outputs nested tree format: { root: { type, props, children } }
      // The Renderer expects flat element map: { root: "id", elements: { id: {...} } }
      const nested = raw.root ?? raw;
      const flat = nestedToFlat(nested);
      return { parsedSpec: flat, error: null };
    } catch (e) {
      return { parsedSpec: null, error: (e as Error).message };
    }
  }, [spec]);

  if (error) {
    return (
      <div className="rounded-md border border-red-300 bg-red-50 p-4 text-red-800 text-sm">
        Failed to render component: {error}
      </div>
    );
  }

  if (!parsedSpec) return null;

  return (
    <div className="my-2 json-render-block">
      <CatalogRenderer spec={parsedSpec} />
    </div>
  );
};
