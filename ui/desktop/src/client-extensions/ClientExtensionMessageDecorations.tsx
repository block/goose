import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import type { Message } from '../types/message';
import { useClientExtensions } from './ClientExtensionsContext';
import { isExtensionToHostMessage } from './extensionHostBridge';
import { buildMessageExtensionContext, extractCodeBlocks } from './messageContext';
import type {
  HostToExtensionMessage,
  MessageRenderPayload,
  RegisteredContentSuffix,
} from './types';

function ClientExtensionRenderSlot({
  extensionId,
  slotId,
  slotKind,
  context,
  payload,
}: {
  extensionId: string;
  slotId: string;
  slotKind: 'contentSuffix' | 'customRender';
  context: ReturnType<typeof buildMessageExtensionContext>;
  payload: MessageRenderPayload;
}) {
  const { getExtensionMainHtml } = useClientExtensions();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState<number | null>(null);
  const [failed, setFailed] = useState(false);

  const handleExtensionMessage = useCallback((event: MessageEvent) => {
    const iframe = iframeRef.current;
    if (!iframe || event.source !== iframe.contentWindow) {
      return;
    }

    if (!isExtensionToHostMessage(event.data)) {
      return;
    }

    if (event.data.type === 'grc/resize') {
      const nextHeight = Math.max(0, Math.min(event.data.height, 480));
      setHeight(nextHeight);
    }
  }, []);

  useEffect(() => {
    window.addEventListener('message', handleExtensionMessage);
    return () => window.removeEventListener('message', handleExtensionMessage);
  }, [handleExtensionMessage]);

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      const html = await getExtensionMainHtml(extensionId);
      if (cancelled) {
        return;
      }

      if (!html) {
        setFailed(true);
        return;
      }

      const iframe = iframeRef.current;
      if (!iframe) {
        return;
      }

      const onLoad = () => {
        const message: HostToExtensionMessage = {
          type: 'grc/render',
          slotId,
          slotKind,
          context,
          payload,
        };
        iframe.contentWindow?.postMessage(message, '*');
      };

      iframe.addEventListener('load', onLoad, { once: true });
      iframe.srcdoc = html;
    })();

    return () => {
      cancelled = true;
    };
  }, [context, extensionId, getExtensionMainHtml, payload, slotId, slotKind]);

  if (failed) {
    return null;
  }

  return (
    <iframe
      ref={iframeRef}
      title={`${extensionId}:${slotId}`}
      sandbox="allow-scripts"
      className="w-full border-0"
      style={{ height: height ?? 24, minHeight: 24 }}
    />
  );
}

export function ClientExtensionMessageDecorations({
  sessionId,
  message,
  displayText,
  imageCount,
}: {
  sessionId: string;
  message: Message;
  displayText: string;
  imageCount: number;
}) {
  const location = useLocation();
  const { getContentSuffixes, getCustomRender, loading, registryVersion } = useClientExtensions();

  const messageContext = useMemo(
    () =>
      buildMessageExtensionContext(sessionId, location.pathname, message, displayText, imageCount),
    [sessionId, location.pathname, message, displayText, imageCount]
  );

  const suffixes = useMemo(
    () => (loading ? [] : getContentSuffixes(messageContext)),
    [getContentSuffixes, loading, messageContext]
  );

  const customRender = useMemo(
    () => (loading ? null : getCustomRender(messageContext, displayText)),
    [getCustomRender, loading, messageContext, displayText]
  );

  const basePayload = useMemo(
    (): MessageRenderPayload => ({
      textPreview: displayText.slice(0, 2000),
      codeBlocks: extractCodeBlocks(displayText),
    }),
    [displayText]
  );

  if (suffixes.length === 0 && !customRender) {
    return null;
  }

  return (
    <div className="mt-2 flex flex-col gap-2 w-full min-w-0">
      {suffixes.map((suffix: RegisteredContentSuffix) => (
        <ClientExtensionRenderSlot
          key={`${registryVersion}:${suffix.extensionId}:${suffix.id}`}
          extensionId={suffix.extensionId}
          slotId={suffix.id}
          slotKind="contentSuffix"
          context={messageContext}
          payload={basePayload}
        />
      ))}
      {customRender && (
        <ClientExtensionRenderSlot
          key={`${registryVersion}:${customRender.extensionId}:${customRender.id}`}
          extensionId={customRender.extensionId}
          slotId={customRender.id}
          slotKind="customRender"
          context={messageContext}
          payload={{
            ...basePayload,
            matchedLanguage: customRender.match.language,
          }}
        />
      )}
    </div>
  );
}
