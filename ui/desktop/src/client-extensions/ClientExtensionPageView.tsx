import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { toastService } from '../toasts';
import { useClientExtensions, useExtensionHostContext } from './ClientExtensionsContext';
import { parseClientExtensionViewPath } from './routes';
import type { ExtensionToHostMessage, HostToExtensionMessage } from './types';

function isExtensionToHostMessage(value: unknown): value is ExtensionToHostMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  const type = (value as { type: unknown }).type;
  return type === 'grc/ui/showMessage' || type === 'grc/chat/setInput';
}

export default function ClientExtensionPageView() {
  const location = useLocation();
  const { extensions, getExtensionMainHtml } = useClientExtensions();
  const hostContext = useExtensionHostContext(null);
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

  useEffect(() => {
    if (!view) {
      setLoadError('Invalid client extension route');
      setHtml(null);
      return;
    }

    if (!extension || !rootLink) {
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
  }, [extension, getExtensionMainHtml, rootLink, view]);

  const handleExtensionMessage = useCallback(
    (event: MessageEvent) => {
      if (event.source !== iframeRef.current?.contentWindow) {
        return;
      }
      if (!isExtensionToHostMessage(event.data)) {
        return;
      }

      switch (event.data.type) {
        case 'grc/ui/showMessage':
          toastService.success({
            title: rootLink?.label ?? 'Extension',
            msg: event.data.text,
          });
          break;
        default:
          break;
      }
    },
    [rootLink?.label]
  );

  useEffect(() => {
    window.addEventListener('message', handleExtensionMessage);
    return () => window.removeEventListener('message', handleExtensionMessage);
  }, [handleExtensionMessage]);

  const notifyActivate = useCallback(() => {
    if (!view || !iframeRef.current?.contentWindow) {
      return;
    }

    const message: HostToExtensionMessage = {
      type: 'grc/activate',
      viewId: view.viewId,
      context: hostContext,
    };
    iframeRef.current.contentWindow.postMessage(message, '*');
  }, [hostContext, view]);

  if (loadError) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-text-secondary">
        {loadError}
      </div>
    );
  }

  if (!html || !rootLink) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-text-secondary">
        Loading extension…
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background-primary">
      <div className="border-b border-border-primary px-4 py-3">
        <h1 className="text-sm font-medium text-text-primary">{rootLink.label}</h1>
      </div>
      <iframe
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
