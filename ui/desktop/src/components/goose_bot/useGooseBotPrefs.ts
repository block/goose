import { useCallback, useEffect, useRef, useState } from 'react';
import { getPrefs, putPrefs } from '../../api/sdk.gen';
import type { GooseBotPrefs } from '../../api/types.gen';

const CACHE_KEY = 'goose-bot:prefs-cache';
const LEGACY_PREFS_KEY = 'goose-bot:preferences';
const LEGACY_INSTRUCTIONS_KEY = 'goose-bot:custom-instructions';
const SYNC_DEBOUNCE_MS = 500;

export type SyncState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'syncing' }
  | { kind: 'synced'; switchboardSynced: boolean }
  | { kind: 'failed'; error: string };

export interface UseGooseBotPrefs {
  prefs: GooseBotPrefs | null;
  update: (patch: Partial<GooseBotPrefs>) => void;
  retry: () => void;
  clearInstall: () => void;
  syncState: SyncState;
}

function loadCachedPrefs(): GooseBotPrefs | null {
  const raw = localStorage.getItem(CACHE_KEY);
  if (raw) {
    try {
      return JSON.parse(raw) as GooseBotPrefs;
    } catch {
      /* fall through */
    }
  }
  return legacyPrefsFromLocalStorage();
}

function legacyPrefsFromLocalStorage(): GooseBotPrefs | null {
  const prefsRaw = localStorage.getItem(LEGACY_PREFS_KEY);
  const instructions = localStorage.getItem(LEGACY_INSTRUCTIONS_KEY);
  if (!prefsRaw && !instructions) return null;
  let parsed: Record<string, unknown> = {};
  if (prefsRaw) {
    try {
      parsed = JSON.parse(prefsRaw) as Record<string, unknown>;
    } catch {
      /* ignore */
    }
  }
  return {
    schema_version: 1,
    auto_review_on_pr_open:
      typeof parsed.autoReviewOnPrOpen === 'boolean' ? parsed.autoReviewOnPrOpen : false,
    trigger_preference: (parsed.triggerPreference as GooseBotPrefs['trigger_preference']) ?? 'pr-open',
    trigger_permission: (parsed.triggerPermission as GooseBotPrefs['trigger_permission']) ?? 'anyone',
    allow_act_on_issues:
      typeof parsed.allowActOnIssues === 'boolean' ? parsed.allowActOnIssues : false,
    allow_commit_on_fix:
      typeof parsed.allowCommitOnFix === 'boolean' ? parsed.allowCommitOnFix : false,
    allow_open_new_prs:
      typeof parsed.allowOpenNewPrs === 'boolean' ? parsed.allowOpenNewPrs : false,
    specific_users_allowlist: Array.isArray(parsed.specificUsersAllowlist)
      ? (parsed.specificUsersAllowlist as string[])
      : [],
    review_severity:
      parsed.exhaustiveReview === true ? 'low' : 'medium',
    custom_instructions: instructions ?? '',
    review_output_style:
      (parsed.reviewOutputStyle as GooseBotPrefs['review_output_style']) ?? 'both',
    review_model_choice:
      (parsed.reviewModelChoice as GooseBotPrefs['review_model_choice']) ?? 'default',
  };
}

function clearLegacyKeys() {
  localStorage.removeItem(LEGACY_PREFS_KEY);
  localStorage.removeItem(LEGACY_INSTRUCTIONS_KEY);
}

function cachePrefs(prefs: GooseBotPrefs) {
  localStorage.setItem(CACHE_KEY, JSON.stringify(prefs));
}

export function clearGooseBotLocalState() {
  localStorage.removeItem(CACHE_KEY);
  clearLegacyKeys();
}

export function useGooseBotPrefs(): UseGooseBotPrefs {
  const [prefs, setPrefs] = useState<GooseBotPrefs | null>(() => loadCachedPrefs());
  const [syncState, setSyncState] = useState<SyncState>({ kind: 'loading' });
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef<GooseBotPrefs | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const { data, error } = await getPrefs();
        if (cancelled) return;
        if (error || !data) throw new Error('failed to load prefs');
        cachePrefs(data);
        setPrefs(data);
        setSyncState({ kind: 'idle' });
      } catch (e) {
        if (cancelled) return;
        const msg = e instanceof Error ? e.message : String(e);
        setSyncState({ kind: 'failed', error: msg });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const flush = useCallback(async () => {
    const target = pendingRef.current;
    if (!target) return;
    setSyncState({ kind: 'syncing' });
    try {
      const { data, error } = await putPrefs({ body: { prefs: target } });
      if (error || !data) {
        throw new Error('PUT /goose-bot/prefs failed');
      }
      cachePrefs(data.prefs);
      if (pendingRef.current === target) {
        setPrefs(data.prefs);
        pendingRef.current = null;
      }
      setSyncState({
        kind: 'synced',
        switchboardSynced: data.switchboard_synced ?? false,
      });
      clearLegacyKeys();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setSyncState({ kind: 'failed', error: msg });
    }
  }, []);

  const update = useCallback(
    (patch: Partial<GooseBotPrefs>) => {
      setPrefs((prev) => {
        if (!prev) return prev;
        const next = { ...prev, ...patch };
        cachePrefs(next);
        pendingRef.current = next;
        return next;
      });
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        void flush();
      }, SYNC_DEBOUNCE_MS);
    },
    [flush]
  );

  const clearInstall = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    pendingRef.current = null;
    clearGooseBotLocalState();
    setPrefs(null);
    setSyncState({ kind: 'idle' });
  }, []);

  const retry = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    if (pendingRef.current) {
      void flush();
    } else if (prefs) {
      pendingRef.current = prefs;
      void flush();
    }
  }, [flush, prefs]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
        if (pendingRef.current) {
          void flush();
        }
      }
    };
  }, [flush]);

  return { prefs, update, retry, clearInstall, syncState };
}
