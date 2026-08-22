type DraftMap = Record<string, string>;

const STORAGE_KEY = 'goose-chat-drafts';
const MAX_DRAFTS = 50;

/**
 * Unsent chat input, kept per chat for the lifetime of the window.
 *
 * sessionStorage rather than localStorage: a draft is meant to survive
 * navigation, not a restart. That is what the implementation removed in #5366
 * did (an in-memory map above the router), and `ChatInput` already keeps its
 * queue state there.
 */
export class SessionDraftStorage {
  private static getDrafts(): DraftMap {
    try {
      const stored = window.sessionStorage.getItem(STORAGE_KEY);
      if (!stored) return {};

      const drafts = JSON.parse(stored) as unknown;
      if (typeof drafts !== 'object' || drafts === null || Array.isArray(drafts)) {
        return {};
      }

      return Object.fromEntries(
        Object.entries(drafts as Record<string, unknown>).filter(
          ([, text]) => typeof text === 'string'
        )
      ) as DraftMap;
    } catch (error) {
      console.error('Error reading chat drafts:', error);
      return {};
    }
  }

  private static setDrafts(drafts: DraftMap) {
    try {
      window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(drafts));
    } catch (error) {
      console.error('Error saving chat drafts:', error);
    }
  }

  static get(key: string): string {
    return this.getDrafts()[key] ?? '';
  }

  static set(key: string, text: string) {
    if (!text) {
      this.clear(key);
      return;
    }

    const drafts = this.getDrafts();
    delete drafts[key];
    drafts[key] = text;

    // Object keys keep insertion order, so the oldest drafts drop out first.
    const keys = Object.keys(drafts);
    if (keys.length > MAX_DRAFTS) {
      for (const stale of keys.slice(0, keys.length - MAX_DRAFTS)) {
        delete drafts[stale];
      }
    }

    this.setDrafts(drafts);
  }

  static clear(key: string) {
    const drafts = this.getDrafts();
    if (!(key in drafts)) return;

    delete drafts[key];
    this.setDrafts(drafts);
  }
}
