import type { ExtensionConfig } from '../api';
import type { FixedExtensionEntry } from '../components/ConfigContext';

export type NextChatExtensionDraft = {
  selectedNames: Set<string>;
};

export function createNextChatExtensionDraft(
  allExtensions: FixedExtensionEntry[] = []
): NextChatExtensionDraft {
  return {
    selectedNames: new Set(
      allExtensions.filter((extension) => extension.enabled).map((extension) => extension.name)
    ),
  };
}

export function selectNextChatExtensions(
  allExtensions: FixedExtensionEntry[],
  draft: NextChatExtensionDraft
): ExtensionConfig[] {
  return allExtensions
    .filter((extension) => draft.selectedNames.has(extension.name))
    .map((extension) => {
      const { enabled: _enabled, ...config } = extension;
      return config as ExtensionConfig;
    });
}

export function isNextChatExtensionSelected(
  extension: FixedExtensionEntry,
  draft: NextChatExtensionDraft
): boolean {
  return draft.selectedNames.has(extension.name);
}

export function toggleNextChatExtension(
  draft: NextChatExtensionDraft,
  extension: FixedExtensionEntry
): NextChatExtensionDraft {
  const selectedNames = new Set(draft.selectedNames);

  if (selectedNames.has(extension.name)) {
    selectedNames.delete(extension.name);
  } else {
    selectedNames.add(extension.name);
  }

  return { selectedNames };
}
