import { useEffect, useRef, useState } from 'react';
import { getInitialWorkingDir } from '../utils/workingDir';

export default function LauncherView() {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // Close launcher when it loses focus
  useEffect(() => {
    const handleBlur = () => {
      window.electron.closeWindow();
    };
    window.addEventListener('blur', handleBlur);
    return () => window.removeEventListener('blur', handleBlur);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      const initialMessage = query;
      setQuery('');
      window.electron.createChatWindow(initialMessage, getInitialWorkingDir());
      setTimeout(() => {
        window.electron.closeWindow();
      }, 200);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Close on Escape
    if (e.key === 'Escape') {
      window.electron.closeWindow();
    }
  };

  return (
    <div className="h-screen w-screen flex bg-transparent overflow-hidden">
      <form
        onSubmit={handleSubmit}
        className="w-full h-full bg-background-default/95 backdrop-blur-lg shadow-2xl border border-border-default"
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full h-full bg-transparent text-text-default text-xl px-6 outline-none placeholder-text-muted"
          placeholder="Ask goose anything..."
          autoFocus
        />
      </form>
    </div>
  );
}
