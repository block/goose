import { useEffect, useState } from 'react';

export function useDarkMode(): boolean {
  const [isDarkMode, setIsDarkMode] = useState<boolean>(() => {
    const html = document.documentElement;
    return (
      html.classList.contains('dark') || window.matchMedia('(prefers-color-scheme: dark)').matches
    );
  });

  useEffect(() => {
    const html = document.documentElement;

    const updateDarkMode = () => {
      setIsDarkMode(html.classList.contains('dark'));
    };

    // Observe class attribute changes
    const observer = new MutationObserver(() => updateDarkMode());
    observer.observe(html, { attributes: true, attributeFilter: ['class'] });

    // Also handle system preference changes (if no dark class is set manually)
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    // eslint-disable-next-line no-undef
    const handleMediaChange = (event: MediaQueryListEvent) => {
      if (!html.classList.contains('dark') && !html.classList.contains('light')) {
        setIsDarkMode(event.matches);
      }
    };
    mediaQuery.addEventListener('change', handleMediaChange);

    // Initial check
    updateDarkMode();

    return () => {
      observer.disconnect();
      mediaQuery.removeEventListener('change', handleMediaChange);
    };
  }, []);

  return isDarkMode;
}
