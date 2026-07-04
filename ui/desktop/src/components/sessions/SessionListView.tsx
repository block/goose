import React, { useCallback, useEffect, useState } from 'react';
import { Share2, Upload } from 'lucide-react';
import { toast } from 'react-toastify';
import { defineMessages, useIntl } from '../../i18n';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { acpImportSession } from '../../acp/sessions';
import { AppEvents } from '../../constants/events';
import { errorMessage } from '../../utils/conversionUtils';
import { getSearchShortcutText } from '../../utils/keyboardShortcuts';
import SessionListPane from './SessionListPane';

const i18n = defineMessages({
  chatHistory: { id: 'sessions.chatHistory', defaultMessage: 'Chat history' },
  chatHistoryDesc: {
    id: 'sessions.chatHistoryDesc',
    defaultMessage: 'View and search your past conversations with Gosling. {shortcut} to search.',
  },
  importSession: { id: 'sessions.import', defaultMessage: 'Import Session' },
  importNostrSession: { id: 'sessions.importNostr', defaultMessage: 'Import Link' },
  importNostrTitle: {
    id: 'sessions.importNostr.title',
    defaultMessage: 'Import Nostr Session',
  },
  importNostrDesc: {
    id: 'sessions.importNostr.description',
    defaultMessage:
      'Paste a Gosling Nostr share link to fetch, decrypt, and import the session.',
  },
  importNostrPlaceholder: {
    id: 'sessions.importNostr.placeholder',
    defaultMessage: 'gosling://sessions/nostr?nevent=...&key=...',
  },
  importing: { id: 'sessions.importing', defaultMessage: 'Importing...' },
  importSuccess: {
    id: 'sessions.toast.imported',
    defaultMessage: 'Session imported successfully',
  },
  importFailed: {
    id: 'sessions.toast.importFailed',
    defaultMessage: 'Failed to import session: {error}',
  },
  cancel: { id: 'sessions.cancel', defaultMessage: 'Cancel' },
  activeTab: { id: 'sessions.tab.active', defaultMessage: 'Active' },
  archivedTab: { id: 'sessions.tab.archived', defaultMessage: 'Archived' },
});

interface SessionListViewProps {
  initialTab?: 'active' | 'archived';
  onSelectSession: (sessionId: string) => void;
  onTabChange?: (tab: 'active' | 'archived') => void;
}

const SessionListView: React.FC<SessionListViewProps> = ({
  initialTab = 'active',
  onSelectSession,
  onTabChange,
}) => {
  const intl = useIntl();
  const [activeTab, setActiveTab] = useState<'active' | 'archived'>(initialTab);
  const [showImportLinkModal, setShowImportLinkModal] = useState(false);
  const [nostrImportLink, setNostrImportLink] = useState('');
  const [isImportingNostr, setIsImportingNostr] = useState(false);
  const [nostrEnabled] = useState(() => window.electron.getConfig().GOSLING_DISABLE_NOSTR_SHARING !== true);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  useEffect(() => {
    setActiveTab(initialTab);
  }, [initialTab]);

  const handleTabChange = useCallback(
    (tab: string) => {
      const nextTab = tab === 'archived' ? 'archived' : 'active';
      setActiveTab(nextTab);
      onTabChange?.(nextTab);
    },
    [onTabChange]
  );

  const notifySessionCreated = useCallback(() => {
    window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
  }, []);

  const handleImportClick = useCallback(async () => {
    const native = window.electron?.selectImportSessionFile;
    if (typeof native === 'function') {
      try {
        const result = await native();
        if (!result) return;
        if (result.error) {
          toast.error(intl.formatMessage(i18n.importFailed, { error: result.error }));
          return;
        }
        await acpImportSession(result.contents, 'json');
        toast.success(intl.formatMessage(i18n.importSuccess));
        notifySessionCreated();
      } catch (error) {
        toast.error(
          intl.formatMessage(i18n.importFailed, {
            error: errorMessage(error, 'Unknown error'),
          })
        );
      }
      return;
    }

    fileInputRef.current?.click();
  }, [intl, notifySessionCreated]);

  const handleImportSession = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file) return;

      try {
        const json = await file.text();
        await acpImportSession(json, 'json');
        toast.success(intl.formatMessage(i18n.importSuccess));
        notifySessionCreated();
      } catch (error) {
        toast.error(
          intl.formatMessage(i18n.importFailed, {
            error: errorMessage(error, 'Unknown error'),
          })
        );
      } finally {
        if (fileInputRef.current) {
          fileInputRef.current.value = '';
        }
      }
    },
    [intl, notifySessionCreated]
  );

  const handleImportNostrLink = useCallback(async () => {
    const deeplink = nostrImportLink.trim();
    if (!deeplink) return;

    setIsImportingNostr(true);
    try {
      await acpImportSession(deeplink, 'nostr');
      setNostrImportLink('');
      setShowImportLinkModal(false);
      toast.success(intl.formatMessage(i18n.importSuccess));
      notifySessionCreated();
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.importFailed, {
          error: errorMessage(error, 'Unknown error'),
        })
      );
    } finally {
      setIsImportingNostr(false);
    }
  }, [intl, nostrImportLink, notifySessionCreated]);

  return (
    <>
      <MainPanelLayout>
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="bg-background-primary px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="mb-1 flex items-center justify-between">
                <h1 className="text-4xl font-light">{intl.formatMessage(i18n.chatHistory)}</h1>
                {activeTab === 'active' && (
                  <div className="flex items-center gap-2">
                    {nostrEnabled && (
                      <Button
                        onClick={() => setShowImportLinkModal(true)}
                        variant="outline"
                        size="sm"
                        className="flex items-center gap-2"
                      >
                        <Share2 className="h-4 w-4" />
                        {intl.formatMessage(i18n.importNostrSession)}
                      </Button>
                    )}
                    <Button
                      onClick={() => void handleImportClick()}
                      variant="outline"
                      size="sm"
                      className="flex items-center gap-2"
                    >
                      <Upload className="h-4 w-4" />
                      {intl.formatMessage(i18n.importSession)}
                    </Button>
                  </div>
                )}
              </div>
              <p className="mb-4 text-sm text-text-secondary">
                {intl.formatMessage(i18n.chatHistoryDesc, { shortcut: getSearchShortcutText() })}
              </p>
              <Tabs value={activeTab} onValueChange={handleTabChange}>
                <TabsList>
                  <TabsTrigger value="active">{intl.formatMessage(i18n.activeTab)}</TabsTrigger>
                  <TabsTrigger value="archived">
                    {intl.formatMessage(i18n.archivedTab)}
                  </TabsTrigger>
                </TabsList>
                <div className="mt-6 flex-1 min-h-0">
                  <TabsContent value="active" forceMount className={activeTab === 'active' ? '' : 'hidden'}>
                    <SessionListPane mode="active" onSelectSession={onSelectSession} />
                  </TabsContent>
                  <TabsContent
                    value="archived"
                    forceMount
                    className={activeTab === 'archived' ? '' : 'hidden'}
                  >
                    <SessionListPane mode="archived" />
                  </TabsContent>
                </div>
              </Tabs>
            </div>
          </div>
        </div>
      </MainPanelLayout>

      <input
        ref={fileInputRef}
        type="file"
        accept=".json,.jsonl,application/json,application/x-ndjson"
        onChange={handleImportSession}
        className="hidden"
      />

      <Dialog open={showImportLinkModal} onOpenChange={setShowImportLinkModal}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Share2 className="h-5 w-5" />
              {intl.formatMessage(i18n.importNostrTitle)}
            </DialogTitle>
            <DialogDescription>{intl.formatMessage(i18n.importNostrDesc)}</DialogDescription>
          </DialogHeader>

          <textarea
            value={nostrImportLink}
            onChange={(event) => setNostrImportLink(event.target.value)}
            placeholder={intl.formatMessage(i18n.importNostrPlaceholder)}
            className="min-h-28 w-full resize-none rounded-lg border border-border-primary bg-background-primary p-3 text-sm text-text-primary outline-none focus:ring-2 focus:ring-border-active"
            disabled={isImportingNostr}
          />

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowImportLinkModal(false)}
              disabled={isImportingNostr}
            >
              {intl.formatMessage(i18n.cancel)}
            </Button>
            <Button
              onClick={() => void handleImportNostrLink()}
              disabled={isImportingNostr || !nostrImportLink.trim()}
            >
              {isImportingNostr ? intl.formatMessage(i18n.importing) : intl.formatMessage(i18n.importSession)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

SessionListView.displayName = 'SessionListView';

export default SessionListView;
