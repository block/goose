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

export interface ContentSuffixContribution {
  id: string;
  when?: string;
}

export interface CustomRenderMatch {
  contentType?: 'code' | 'text';
  language?: string;
}

export interface CustomRenderContribution {
  id: string;
  match: CustomRenderMatch;
  display?: 'inline';
  priority?: number;
  when?: string;
}

export interface SidecarContribution {
  id: string;
  label: string;
  when?: string;
  defaultOpen?: boolean;
}

export interface ClientExtensionContributes {
  chatActions?: ChatActionContribution[];
  rootLinks?: RootLinkContribution[];
  contentSuffixes?: ContentSuffixContribution[];
  customRenders?: CustomRenderContribution[];
  sidecars?: SidecarContribution[];
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

export interface RegisteredContentSuffix extends ContentSuffixContribution {
  extensionId: string;
}

export interface RegisteredCustomRender extends CustomRenderContribution {
  extensionId: string;
}

export interface RegisteredSidecar extends SidecarContribution {
  extensionId: string;
}

export interface CodeBlock {
  language: string;
  content: string;
}

export interface ExtensionHostContext {
  sessionId: string | null;
  route: string;
}

export interface MessageExtensionHostContext extends ExtensionHostContext {
  messageId: string | null;
  role: string;
  hasText: boolean;
  hasImage: boolean;
  hasToolRequests: boolean;
  codeLanguages: string[];
}

export interface MessageRenderPayload {
  textPreview: string;
  codeBlocks: CodeBlock[];
  matchedLanguage?: string;
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
      viewKind?: 'rootLink' | 'sidecar';
      context: ExtensionHostContext;
    }
  | {
      type: 'grc/render';
      slotId: string;
      slotKind: 'contentSuffix' | 'customRender';
      context: MessageExtensionHostContext;
      payload: MessageRenderPayload;
    };

export type ExtensionToHostMessage =
  | { type: 'grc/ui/showMessage'; text: string }
  | { type: 'grc/chat/setInput'; text: string }
  | { type: 'grc/resize'; height: number };
