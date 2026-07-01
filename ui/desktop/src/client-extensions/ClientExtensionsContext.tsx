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
  type RegisteredSidecar,
} from './types';
import { evaluateWhenClause } from './when';
import { clientExtensionViewPath } from './routes';
import { selectCustomRender } from './customRender';
import { extractCodeBlocks } from './messageContext';

interface ClientExtensionsContextValue {
  extensions: DiscoveredClientExtension[];
  enabledExtensions: DiscoveredClientExtension[];
  loading: boolean;
  registryVersion: number;
  getChatActions: (context: ExtensionHostContext) => RegisteredChatAction[];
  getRootLinks: (context: ExtensionHostContext) => RegisteredRootLink[];
  getContentSuffixes: (context: MessageExtensionHostContext) => RegisteredContentSuffix[];
  getCustomRender: (
    context: MessageExtensionHostContext,
    displayText: string
  ) => RegisteredCustomRender | null;
  getSidecars: (context: ExtensionHostContext) => RegisteredSidecar[];
  getExtensionMainHtml: (extensionId: string) => Promise<string | null>;
  reloadExtensions: () => Promise<void>;
  setExtensionEnabled: (extensionId: string, enabled: boolean) => Promise<void>;
  uninstallExtension: (extensionId: string) => Promise<void>;
  installExtension: (sourcePath: string) => Promise<void>;
}

const ClientExtensionsContext = createContext<ClientExtensionsContextValue | null>(null);

export function ClientExtensionsProvider({ children }: { children: React.ReactNode }) {
  const [extensions, setExtensions] = useState<DiscoveredClientExtension[]>([]);
  const [loading, setLoading] = useState(true);
  const [registryVersion, setRegistryVersion] = useState(0);

  const applyDiscovered = useCallback((discovered: DiscoveredClientExtension[]) => {
    setExtensions(discovered);
    setRegistryVersion((version) => version + 1);
  }, []);

  const reloadExtensions = useCallback(async () => {
    try {
      const discovered = await window.electron.listClientExtensions();
      applyDiscovered(discovered);
    } catch (error) {
      console.warn('[client-extensions] Failed to discover extensions:', error);
      applyDiscovered([]);
    } finally {
      setLoading(false);
    }
  }, [applyDiscovered]);

  const enabledExtensions = useMemo(
    () => extensions.filter((extension) => extension.enabled),
    [extensions]
  );

  const setExtensionEnabled = useCallback(
    async (extensionId: string, enabled: boolean) => {
      try {
        const discovered = await window.electron.setClientExtensionEnabled(extensionId, enabled);
        applyDiscovered(discovered);
      } catch (error) {
        console.warn('[client-extensions] Failed to update extension state:', error);
      }
    },
    [applyDiscovered]
  );

  const uninstallExtension = useCallback(
    async (extensionId: string) => {
      try {
        const discovered = await window.electron.uninstallClientExtension(extensionId);
        applyDiscovered(discovered);
      } catch (error) {
        console.warn('[client-extensions] Failed to uninstall extension:', error);
        throw error;
      }
    },
    [applyDiscovered]
  );

  const installExtension = useCallback(
    async (sourcePath: string) => {
      try {
        const discovered = await window.electron.installClientExtension(sourcePath);
        applyDiscovered(discovered);
      } catch (error) {
        console.warn('[client-extensions] Failed to install extension:', error);
        throw error;
      }
    },
    [applyDiscovered]
  );

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

      for (const extension of enabledExtensions) {
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
    [enabledExtensions]
  );

  const getRootLinks = useCallback(
    (context: ExtensionHostContext): RegisteredRootLink[] => {
      const links: RegisteredRootLink[] = [];

      for (const extension of enabledExtensions) {
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
    [enabledExtensions]
  );

  const getContentSuffixes = useCallback(
    (context: MessageExtensionHostContext): RegisteredContentSuffix[] => {
      const suffixes: RegisteredContentSuffix[] = [];

      for (const extension of enabledExtensions) {
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
    [enabledExtensions]
  );

  const getCustomRender = useCallback(
    (context: MessageExtensionHostContext, displayText: string): RegisteredCustomRender | null => {
      const renders: RegisteredCustomRender[] = [];

      for (const extension of enabledExtensions) {
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
    [enabledExtensions]
  );

  const getSidecars = useCallback(
    (context: ExtensionHostContext): RegisteredSidecar[] => {
      const sidecars: RegisteredSidecar[] = [];

      for (const extension of enabledExtensions) {
        const contributions = extension.manifest.contributes?.sidecars ?? [];
        for (const contribution of contributions) {
          if (!evaluateWhenClause(contribution.when, context)) {
            continue;
          }
          sidecars.push({
            ...contribution,
            extensionId: extension.id,
          });
        }
      }

      return sidecars;
    },
    [enabledExtensions]
  );

  const value = useMemo(
    () => ({
      extensions,
      enabledExtensions,
      loading,
      registryVersion,
      getChatActions,
      getRootLinks,
      getContentSuffixes,
      getCustomRender,
      getSidecars,
      getExtensionMainHtml,
      reloadExtensions,
      setExtensionEnabled,
      uninstallExtension,
      installExtension,
    }),
    [
      extensions,
      enabledExtensions,
      loading,
      registryVersion,
      getChatActions,
      getRootLinks,
      getContentSuffixes,
      getCustomRender,
      getSidecars,
      getExtensionMainHtml,
      reloadExtensions,
      setExtensionEnabled,
      uninstallExtension,
      installExtension,
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
