import { useMemo, useRef, useState } from 'react';
import { getConfiguredProductName } from '../branding/productText';
import { SecurityExtensionOverview, SecurityTaskExtensionHints } from './security/SecurityExtensionHints';
import { defineMessages, useIntl } from '../i18n';
import { SECURITY_TASK_IDS, resolveSecurityTaskLaunchConfig } from '../security/taskCatalog';
import { SECURITY_TASK_COPY, securityTaskUiMessages } from '../security/taskMessages';
import { getInitialWorkingDir } from '../utils/workingDir';

const messages = defineMessages({
  placeholder: {
    id: 'launcher.placeholder',
    defaultMessage: 'Ask {appName} anything...',
  },
});

export default function LauncherView() {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const intl = useIntl();
  const appName = getConfiguredProductName();
  const launcherTasks = useMemo(
    () =>
      SECURITY_TASK_IDS.map((taskId) => ({
        taskId,
        copy: SECURITY_TASK_COPY[taskId],
        launch: resolveSecurityTaskLaunchConfig(taskId, intl.locale),
      })),
    [intl.locale]
  );

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

  const handleTaskLaunch = (taskId: (typeof SECURITY_TASK_IDS)[number]) => {
    const task = resolveSecurityTaskLaunchConfig(taskId, intl.locale);
    window.electron.createChatWindow({
      dir: getInitialWorkingDir(),
      query: task.starterPrompt,
      recipeId: task.recipeId,
    });
    setTimeout(() => {
      window.electron.closeWindow();
    }, 200);
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
        className="w-full h-full bg-background-primary/95 backdrop-blur-lg shadow-2xl border border-border-primary p-3 flex flex-col gap-3"
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full rounded-xl bg-transparent text-text-primary text-xl px-5 py-4 outline-none placeholder:text-text-secondary border border-border-primary"
          placeholder={intl.formatMessage(messages.placeholder, { appName })}
          autoFocus
        />

        <div className="flex-1 min-h-0">
          <div className="mb-2 px-1 space-y-2" data-testid="launcher-security-tasks">
            <div className="flex items-center justify-between gap-4">
              <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-secondary">
                {intl.formatMessage(securityTaskUiMessages.launcherSectionTitle)}
              </p>
              <p className="text-xs text-right text-text-secondary">
                {intl.formatMessage(securityTaskUiMessages.launcherSectionDescription)}
              </p>
            </div>

            <SecurityExtensionOverview />
          </div>

          <div className="grid grid-cols-2 gap-2">
            {launcherTasks.map(({ taskId, copy, launch }) => (
              <button
                key={taskId}
                type="button"
                onClick={() => handleTaskLaunch(taskId)}
                data-testid={`launcher-security-task-${taskId}`}
                className="rounded-xl border border-border-primary bg-background-secondary/70 px-3 py-3 text-left transition-colors hover:bg-background-secondary"
              >
                <div className="flex items-start justify-between gap-2 mb-1">
                  <span className="text-sm font-medium text-text-primary">
                    {intl.formatMessage(copy.title)}
                  </span>
                  <span
                    data-testid={`launcher-security-task-badge-${taskId}`}
                    className="rounded-full border border-border-primary px-2 py-0.5 text-[10px] uppercase tracking-[0.12em] text-text-secondary"
                  >
                    {intl.formatMessage(
                      launch.availability === 'ready'
                        ? securityTaskUiMessages.badgeReady
                        : securityTaskUiMessages.badgePreview
                    )}
                  </span>
                </div>
                <p className="text-xs leading-5 text-text-secondary">
                  {intl.formatMessage(copy.description)}
                </p>

                <SecurityTaskExtensionHints
                  extensionIds={launch.recommendedExtensionIds}
                  compact={true}
                />
              </button>
            ))}
          </div>
        </div>
      </form>
    </div>
  );
}
