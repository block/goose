import { Session, startAgent, ExtensionConfig } from './api';
import type { setViewType } from './hooks/useNavigation';
import {
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
  hasExtensionOverrides,
} from './store/extensionOverrides';
import type { FixedExtensionEntry } from './components/ConfigContext';
import { AppEvents } from './constants/events';
import { decodeRecipe, Recipe } from './recipe';
import type { WhiteLabelConfig } from './whitelabel/types';
import { DEFAULT_WHITELABEL_CONFIG } from './whitelabel/defaults';
import { RESOURCES_PREFIX } from './whitelabel/constants';

function getWhiteLabelConfig(): WhiteLabelConfig {
  try {
    return __WHITELABEL_CONFIG__;
  } catch {
    return DEFAULT_WHITELABEL_CONFIG;
  }
}

function resolveResourcePath(p: string): string {
  if (!p.startsWith(RESOURCES_PREFIX)) return p;
  const resourcesPath = (window.appConfig?.get('GOOSE_RESOURCES_PATH') as string) || '';
  return p.replace(RESOURCES_PREFIX, resourcesPath + '/whitelabel-resources');
}

/**
 * Build the system prompt override from the whitelabel config.
 * This replaces system.md entirely — the agent becomes whatever the
 * whitelabel config says it is. The override is a Tera template with
 * access to {{ extensions }}, {{ current_date_time }}, etc.
 *
 * Returns undefined if no whitelabel customization is configured.
 */
export function buildWhiteLabelSystemPrompt(): string | undefined {
  const config = getWhiteLabelConfig();
  const { defaults } = config;

  if (!defaults.systemPrompt && !defaults.tools?.length) {
    return undefined;
  }

  const parts: string[] = [];

  // Base system prompt — the agent persona
  if (defaults.systemPrompt) {
    parts.push(defaults.systemPrompt.trim());
  }

  // Tools
  if (defaults.tools && defaults.tools.length > 0) {
    const lines = ['# Tools', ''];
    for (const tool of defaults.tools) {
      lines.push(`## \`${tool.name}\``);
      lines.push(tool.description);
      lines.push(`Binary: \`${resolveResourcePath(tool.path)}\``);
      if (tool.env) {
        const envList = Object.entries(tool.env)
          .map(([k, v]) => `  - \`${k}\`: ${v || '(required)'}`)
          .join('\n');
        lines.push(`Environment variables:\n${envList}`);
      }
      if (tool.helpText) {
        lines.push('');
        lines.push(tool.helpText);
      }
      lines.push('');
    }
    parts.push(lines.join('\n'));
  }

  if (parts.length === 0) return undefined;

  return parts.join('\n\n');
}

export function shouldShowNewChatTitle(session: Session): boolean {
  if (session.recipe) {
    return false;
  }
  return !session.user_set_name && session.message_count === 0;
}

export function resumeSession(session: Session, setView: setViewType) {
  const eventDetail = {
    sessionId: session.id,
    initialMessage: undefined,
  };

  window.dispatchEvent(
    new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function createSession(
  workingDir: string,
  options?: {
    recipeDeeplink?: string;
    recipeId?: string;
    extensionConfigs?: ExtensionConfig[];
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const body: {
    working_dir: string;
    recipe?: Recipe;
    recipe_id?: string;
    extension_overrides?: ExtensionConfig[];
    system_prompt?: string;
  } = {
    working_dir: workingDir,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe = await decodeRecipe(options.recipeDeeplink);
  }

  // Apply whitelabel system prompt override (replaces system.md entirely)
  const wlSystemPrompt = buildWhiteLabelSystemPrompt();
  if (wlSystemPrompt) {
    body.system_prompt = wlSystemPrompt;
  }

  if (options?.extensionConfigs && options.extensionConfigs.length > 0) {
    body.extension_overrides = options.extensionConfigs;
  } else if (options?.allExtensions) {
    const extensionConfigs = getExtensionConfigsWithOverrides(options.allExtensions);
    if (extensionConfigs.length > 0) {
      body.extension_overrides = extensionConfigs;
    }
    if (hasExtensionOverrides()) {
      clearExtensionOverrides();
    }
  }

  const newAgent = await startAgent({
    body,
    throwOnError: true,
  });
  return newAgent.data;
}

export async function startNewSession(
  initialText: string | undefined,
  setView: setViewType,
  workingDir: string,
  options?: {
    recipeDeeplink?: string;
    recipeId?: string;
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const session = await createSession(workingDir, options);
  window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED, { detail: { session } }));

  const initialMessage = initialText ? { msg: initialText, images: [] } : undefined;

  const eventDetail = {
    sessionId: session.id,
    initialMessage,
  };

  window.dispatchEvent(
    new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    initialMessage,
    resumeSessionId: session.id,
  });
  return session;
}
