import { useRef, useState } from 'react';
import { getInitialWorkingDir } from '../utils/workingDir';

export default function LauncherView() {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      const initialMessage = query;
      setQuery('');
      window.electron.createChatWindow({ query: initialMessage, dir: getInitialWorkingDir() });
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
        className="w-full h-full bg-background-primary/95 backdrop-blur-lg shadow-2xl border border-border-primary"
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full h-full bg-transparent text-text-primary text-xl px-6 outline-none placeholder:text-text-secondary"
          placeholder="Ask goose anything..."
          autoFocus
        />
      </form>
    </div>
  );
}
