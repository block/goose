import React, { useEffect } from 'react';
import { useLocation } from '@docusaurus/router';

export default function ExtensionRedirect(): JSX.Element {
  const location = useLocation();

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    window.location.href = `goose://extension${params.toString() ? '?' + params.toString() : ''}`;
  }, [location]);

  return (
    <div style={{ padding: '2rem', textAlign: 'center' }}>
      Redirecting to Goose...
    </div>
  );
}
