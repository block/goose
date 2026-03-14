import React, { useEffect } from 'react';
import type { ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

export default function Root({ children }: Props): JSX.Element {
  // Initialize gtag as no-op if not present (prevents errors in development)
  useEffect(() => {
    if (typeof window !== 'undefined' && !window.gtag) {
      (window as any).gtag = function() {};
    }
  }, []);

  return <>{children}</>;
}
