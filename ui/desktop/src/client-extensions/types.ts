export const CLIENT_EXTENSION_MANIFEST = 'client-extension.json';

export interface ChatActionContribution {
  id: string;
  label: string;
  when?: string;
}

export interface RootLinkContribution {
  id: string;
  label: string;
  when?: string;
}

export interface ClientExtensionContributes {
  chatActions?: ChatActionContribution[];
  rootLinks?: RootLinkContribution[];
}

export interface ClientExtensionManifest {
  id: string;
  version: string;
  engines?: {
    grc?: string;
  };
  main: string;
  contributes?: ClientExtensionContributes;
}

export interface DiscoveredClientExtension {
  id: string;
  rootPath: string;
  manifest: ClientExtensionManifest;
}

export interface RegisteredChatAction extends ChatActionContribution {
  extensionId: string;
}

export interface RegisteredRootLink extends RootLinkContribution {
  extensionId: string;
  path: string;
}

export interface ExtensionHostContext {
  sessionId: string | null;
  route: string;
}

export type HostToExtensionMessage =
  | {
      type: 'grc/action';
      actionId: string;
      context: ExtensionHostContext;
    }
  | {
      type: 'grc/activate';
      viewId: string;
      context: ExtensionHostContext;
    };

export type ExtensionToHostMessage =
  | { type: 'grc/ui/showMessage'; text: string }
  | { type: 'grc/chat/setInput'; text: string };
