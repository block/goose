import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import {
  type DiscoveredClientExtension,
  type ExtensionHostContext,
  type MessageExtensionHostContext,
  type RegisteredChatAction,
  type RegisteredContentSuffix,
  type RegisteredCustomRender,
  type RegisteredRootLink,
} from './types';
import { evaluateWhenClause } from './when';
import { clientExtensionViewPath } from './routes';
import { selectCustomRender } from './customRender';
import { extractCodeBlocks } from './messageContext';

interface ClientExtensionsContextValue {
  extensions: DiscoveredClientExtension[];
  loading: boolean;
  getChatActions: (context: ExtensionHostContext) => RegisteredChatAction[];
  getRootLinks: (context: ExtensionHostContext) => RegisteredRootLink[];
  getContentSuffixes: (context: MessageExtensionHostContext) => RegisteredContentSuffix[];
  getCustomRender: (
    context: MessageExtensionHostContext,
    displayText: string
  ) => RegisteredCustomRender | null;
  getExtensionMainHtml: (extensionId: string) => Promise<string | null>;
  reloadExtensions: () => Promise<void>;
}

const ClientExtensionsContext = createContext<ClientExtensionsContextValue | null>(null);

export function ClientExtensionsProvider({ children }: { children: React.ReactNode }) {
  const [extensions, setExtensions] = useState<DiscoveredClientExtension[]>([]);
  const [loading, setLoading] = useState(true);

  const reloadExtensions = useCallback(async () => {
    try {
      const discovered = await window.electron.listClientExtensions();
      setExtensions(discovered);
    } catch (error) {
      console.warn('[client-extensions] Failed to discover extensions:', error);
      setExtensions([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reloadExtensions();
  }, [reloadExtensions]);

  const getExtensionMainHtml = useCallback(async (extensionId: string) => {
    try {
      return await window.electron.readClientExtensionMain(extensionId);
    } catch (error) {
      console.warn(`[client-extensions] Failed to read main for "${extensionId}":`, error);
      return null;
    }
  }, []);

  const getChatActions = useCallback(
    (context: ExtensionHostContext): RegisteredChatAction[] => {
      const actions: RegisteredChatAction[] = [];

      for (const extension of extensions) {
        const contributions = extension.manifest.contributes?.chatActions ?? [];
        for (const contribution of contributions) {
          if (!evaluateWhenClause(contribution.when, context)) {
            continue;
          }
          actions.push({
            ...contribution,
            extensionId: extension.id,
          });
        }
      }

      return actions;
    },
    [extensions]
  );

  const getRootLinks = useCallback(
    (context: ExtensionHostContext): RegisteredRootLink[] => {
      const links: RegisteredRootLink[] = [];

      for (const extension of extensions) {
        const contributions = extension.manifest.contributes?.rootLinks ?? [];
        for (const contribution of contributions) {
          if (!evaluateWhenClause(contribution.when, context)) {
            continue;
          }
          links.push({
            ...contribution,
            extensionId: extension.id,
            path: clientExtensionViewPath(extension.id, contribution.id),
          });
        }
      }

      return links;
    },
    [extensions]
  );

  const getContentSuffixes = useCallback(
    (context: MessageExtensionHostContext): RegisteredContentSuffix[] => {
      const suffixes: RegisteredContentSuffix[] = [];

      for (const extension of extensions) {
        const contributions = extension.manifest.contributes?.contentSuffixes ?? [];
        for (const contribution of contributions) {
          if (!evaluateWhenClause(contribution.when, context)) {
            continue;
          }
          suffixes.push({
            ...contribution,
            extensionId: extension.id,
          });
        }
      }

      return suffixes;
    },
    [extensions]
  );

  const getCustomRender = useCallback(
    (context: MessageExtensionHostContext, displayText: string): RegisteredCustomRender | null => {
      const renders: RegisteredCustomRender[] = [];

      for (const extension of extensions) {
        const contributions = extension.manifest.contributes?.customRenders ?? [];
        for (const contribution of contributions) {
          renders.push({
            ...contribution,
            extensionId: extension.id,
          });
        }
      }

      return selectCustomRender(renders, context, extractCodeBlocks(displayText));
    },
    [extensions]
  );

  const value = useMemo(
    () => ({
      extensions,
      loading,
      getChatActions,
      getRootLinks,
      getContentSuffixes,
      getCustomRender,
      getExtensionMainHtml,
      reloadExtensions,
    }),
    [
      extensions,
      loading,
      getChatActions,
      getRootLinks,
      getContentSuffixes,
      getCustomRender,
      getExtensionMainHtml,
      reloadExtensions,
    ]
  );

  return (
    <ClientExtensionsContext.Provider value={value}>{children}</ClientExtensionsContext.Provider>
  );
}

export function useClientExtensions(): ClientExtensionsContextValue {
  const context = useContext(ClientExtensionsContext);
  if (!context) {
    throw new Error('useClientExtensions must be used within ClientExtensionsProvider');
  }
  return context;
}

export function useExtensionHostContext(sessionId: string | null): ExtensionHostContext {
  const location = useLocation();
  return useMemo(
    () => ({
      sessionId,
      route: location.pathname,
    }),
    [sessionId, location.pathname]
  );
}
