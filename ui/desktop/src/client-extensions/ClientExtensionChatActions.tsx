import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '../components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../components/ui/Tooltip';
import { cn } from '../utils';
import { toastService } from '../toasts';
import { useClientExtensions, useExtensionHostContext } from './ClientExtensionsContext';
import { isExtensionToHostMessage } from './extensionHostBridge';
import type { HostToExtensionMessage, RegisteredChatAction } from './types';

interface ExtensionRuntime {
  iframe: HTMLIFrameElement;
  ready: boolean;
}

function ClientExtensionActionButton({
  action,
  hostContext,
  onSetInput,
}: {
  action: RegisteredChatAction;
  hostContext: ReturnType<typeof useExtensionHostContext>;
  onSetInput?: (text: string) => void;
}) {
  const { getExtensionMainHtml } = useClientExtensions();
  const runtimeRef = useRef<ExtensionRuntime | null>(null);
  const [activating, setActivating] = useState(false);

  const handleExtensionMessage = useCallback(
    (event: MessageEvent) => {
      const runtime = runtimeRef.current;
      if (!runtime || event.source !== runtime.iframe.contentWindow) {
        return;
      }

      if (!isExtensionToHostMessage(event.data)) {
        return;
      }

      switch (event.data.type) {
        case 'grc/ui/showMessage':
          toastService.success({ title: action.label, msg: event.data.text });
          break;
        case 'grc/chat/setInput':
          onSetInput?.(event.data.text);
          break;
        default:
          break;
      }
    },
    [action.label, onSetInput]
  );

  useEffect(() => {
    window.addEventListener('message', handleExtensionMessage);
    return () => window.removeEventListener('message', handleExtensionMessage);
  }, [handleExtensionMessage]);

  const ensureRuntime = useCallback(async () => {
    if (runtimeRef.current) {
      return runtimeRef.current;
    }

    const html = await getExtensionMainHtml(action.extensionId);
    if (!html) {
      return null;
    }

    const iframe = document.createElement('iframe');
    iframe.title = `${action.extensionId} runtime`;
    iframe.sandbox.add('allow-scripts');
    iframe.setAttribute('aria-hidden', 'true');
    iframe.style.cssText = 'position:absolute;width:0;height:0;border:0;visibility:hidden';
    iframe.srcdoc = html;
    document.body.appendChild(iframe);

    const runtime: ExtensionRuntime = { iframe, ready: false };
    runtimeRef.current = runtime;

    await new Promise<void>((resolve) => {
      const onLoad = () => {
        runtime.ready = true;
        resolve();
      };
      iframe.addEventListener('load', onLoad, { once: true });
    });

    return runtime;
  }, [action.extensionId, getExtensionMainHtml]);

  useEffect(() => {
    return () => {
      runtimeRef.current?.iframe.remove();
      runtimeRef.current = null;
    };
  }, []);

  const onClick = useCallback(async () => {
    if (activating) {
      return;
    }

    setActivating(true);
    try {
      const runtime = await ensureRuntime();
      if (!runtime?.iframe.contentWindow) {
        toastService.error({
          title: action.label,
          msg: 'Extension failed to load',
        });
        return;
      }

      const message: HostToExtensionMessage = {
        type: 'grc/action',
        actionId: action.id,
        context: hostContext,
      };
      runtime.iframe.contentWindow.postMessage(message, '*');
    } catch (error) {
      console.warn('[client-extensions] Action failed:', error);
      toastService.error({
        title: action.label,
        msg: 'Extension action failed',
      });
    } finally {
      setActivating(false);
    }
  }, [action.id, action.label, activating, ensureRuntime, hostContext]);

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          shape="round"
          disabled={activating}
          onClick={() => void onClick()}
          className={cn(
            'text-text-primary/70 hover:text-text-primary transition-colors',
            activating && 'opacity-50'
          )}
        >
          <span className="text-xs font-medium px-0.5">{action.label}</span>
        </Button>
      </TooltipTrigger>
      <TooltipContent>{action.label}</TooltipContent>
    </Tooltip>
  );
}

export function ClientExtensionChatActions({
  sessionId,
  onSetInput,
}: {
  sessionId: string | null;
  onSetInput?: (text: string) => void;
}) {
  const hostContext = useExtensionHostContext(sessionId);
  const { getChatActions, loading, registryVersion } = useClientExtensions();
  const actions = getChatActions(hostContext);

  if (loading || actions.length === 0) {
    return null;
  }

  return (
    <>
      {actions.map((action) => (
        <ClientExtensionActionButton
          key={`${registryVersion}:${action.extensionId}:${action.id}`}
          action={action}
          hostContext={hostContext}
          onSetInput={onSetInput}
        />
      ))}
    </>
  );
}
