import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import {
  isExtensionToHostMessage,
  notifyExtensionActivate,
  routeExtensionToHostMessage,
} from './extensionHostBridge';
import { useClientExtensions, useExtensionHostContext } from './ClientExtensionsContext';
import { useWindowMessage } from '../hooks/useWindowMessage';
import { parseClientExtensionViewPath } from './routes';
import type { HostToExtensionMessage } from './types';
import { useNavigationSessions } from '../hooks/useNavigationSessions';
import { Button } from '../components/ui/button';

export default function ClientExtensionPageView() {
  const location = useLocation();
  const { extensions, getExtensionMainHtml, registryVersion } = useClientExtensions();
  const hostContext = useExtensionHostContext(null);
  const { handleNavClick } = useNavigationSessions();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [html, setHtml] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const view = useMemo(
    () => parseClientExtensionViewPath(location.pathname),
    [location.pathname]
  );

  const extension = useMemo(
    () => (view ? extensions.find((entry) => entry.id === view.extensionId) : undefined),
    [extensions, view]
  );

  const rootLink = useMemo(() => {
    if (!view || !extension) {
      return undefined;
    }
    return extension.manifest.contributes?.rootLinks?.find((link) => link.id === view.viewId);
  }, [extension, view]);

  const postToExtension = useCallback((payload: unknown) => {
    iframeRef.current?.contentWindow?.postMessage(payload, '*');
  }, []);

  useEffect(() => {
    if (!view) {
      setLoadError('Invalid client extension route');
      setHtml(null);
      return;
    }

    if (!extension || !extension.enabled || !rootLink) {
      setLoadError(`Extension view not found: ${view.extensionId}/${view.viewId}`);
      setHtml(null);
      return;
    }

    let cancelled = false;
    setLoadError(null);
    setHtml(null);

    void getExtensionMainHtml(view.extensionId).then((content) => {
      if (cancelled) {
        return;
      }
      if (!content) {
        setLoadError(`Failed to load extension "${view.extensionId}"`);
        return;
      }
      setHtml(content);
    });

    return () => {
      cancelled = true;
    };
  }, [extension, getExtensionMainHtml, registryVersion, rootLink, view]);

  const handleExtensionMessage = useCallback(
    async (event: MessageEvent) => {
      if (event.source !== iframeRef.current?.contentWindow || !view || !extension) {
        return;
      }
      if (!isExtensionToHostMessage(event.data)) {
        return;
      }

      await routeExtensionToHostMessage(
        view.extensionId,
        extension.manifest.hostCapabilities,
        event.data,
        postToExtension,
        rootLink?.label ?? 'Extension'
      );
    },
    [extension, postToExtension, rootLink?.label, view]
  );

  useWindowMessage(handleExtensionMessage);

  const notifyActivate = useCallback(() => {
    if (!view) {
      return;
    }

    const message: HostToExtensionMessage = {
      type: 'grc/activate',
      viewId: view.viewId,
      viewKind: 'rootLink',
      context: hostContext,
    };
    notifyExtensionActivate(
      iframeRef.current,
      message,
      extension?.manifest.hostCapabilities
    );
  }, [extension?.manifest.hostCapabilities, hostContext, view]);

  if (loadError) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-text-secondary">
        {loadError}
      </div>
    );
  }

  if (!html || !rootLink || !view) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-text-secondary">
        Loading extension…
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background-primary">
      <div className="flex items-center gap-2 border-b border-border-primary px-4 py-3">
        <Button
          type="button"
          variant="ghost"
          size="xs"
          onClick={() => handleNavClick('/pair')}
          className="no-drag"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to chat
        </Button>
        <h1 className="text-sm font-medium text-text-primary">{rootLink.label}</h1>
      </div>
      <iframe
        key={`${registryVersion}:${view.extensionId}:${view.viewId}`}
        ref={iframeRef}
        title={rootLink.label}
        sandbox="allow-scripts"
        srcDoc={html}
        onLoad={notifyActivate}
        className="h-full w-full flex-1 border-0 bg-background-primary"
      />
    </div>
  );
}
