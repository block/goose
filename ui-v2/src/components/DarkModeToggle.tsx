import { useEffect, useState, ReactElement } from 'react';

import { Moon, Sun } from 'lucide-react';

export function DarkModeToggle(): ReactElement {
  const [isDark, setIsDark] = useState(false);

  useEffect(() => {
    // Initialize from localStorage or system preference
    const savedTheme = localStorage.getItem('theme');
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

    const shouldBeDark = savedTheme === 'dark' || (!savedTheme && systemPrefersDark);
    setIsDark(shouldBeDark);

    if (shouldBeDark) {
      document.documentElement.classList.add('dark');
    }
  }, []);

  const toggleDarkMode = () => {
    const newIsDark = !isDark;
    setIsDark(newIsDark);

    if (newIsDark) {
      document.documentElement.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else {
      document.documentElement.classList.remove('dark');
      localStorage.setItem('theme', 'light');
    }
  };

  return (
    <button
      onClick={toggleDarkMode}
      className="
        fixed bottom-4 left-4 z-50
        w-10 h-10
        rounded-full 
        cursor-pointer
        flex items-center justify-center
        transition-all duration-200
        bg-white/80 dark:bg-black/80
        backdrop-blur-sm
        shadow-[0_0_13.7px_rgba(0,0,0,0.04)]
        dark:shadow-[0_0_24px_rgba(255,255,255,0.08)]
        hover:bg-white dark:hover:bg-black
        text-black/80 dark:text-white/80
        hover:text-black dark:hover:text-white
        group
      "
      title={isDark ? 'Switch to Light Mode' : 'Switch to Dark Mode'}
    >
      <span className="block dark:hidden transform transition-transform group-hover:rotate-12">
        <Moon size={18} />
      </span>
      <span className="hidden dark:block transform transition-transform group-hover:rotate-12">
        <Sun size={18} />
      </span>
    </button>
  );
}
