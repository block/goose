import { Session, startAgent, ExtensionConfig, readConfig, setConfigProvider } from './api';
import type { setViewType } from './hooks/useNavigation';
import {
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
  hasExtensionOverrides,
} from './store/extensionOverrides';
import type { FixedExtensionEntry } from './components/ConfigContext';
import { AppEvents } from './constants/events';
import { decodeRecipe, Recipe } from './recipe';
import { getConfiguredDefaultPredefinedModel } from './components/settings/models/predefinedModelsUtils';

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

function getDesktopFallbackModelAndProvider(): { provider: string; model: string } {
  const defaultPredefinedModel = getConfiguredDefaultPredefinedModel();
  const configuredProvider = window.appConfig?.get('GOOSE_DEFAULT_PROVIDER');
  const configuredModel = window.appConfig?.get('GOOSE_DEFAULT_MODEL');

  return {
    provider:
      typeof configuredProvider === 'string' && configuredProvider.trim()
        ? configuredProvider.trim()
        : (defaultPredefinedModel?.provider ?? ''),
    model:
      typeof configuredModel === 'string' && configuredModel.trim()
        ? configuredModel.trim()
        : (defaultPredefinedModel?.name ?? ''),
  };
}

async function readConfigString(key: string): Promise<string> {
  const response = await readConfig({
    body: {
      key,
      is_secret: false,
    },
  });

  return typeof response.data === 'string' ? response.data.trim() : '';
}

export async function ensureSessionProviderAndModelConfigured(): Promise<void> {
  const [currentProvider, currentModel] = await Promise.all([
    readConfigString('GOOSE_PROVIDER'),
    readConfigString('GOOSE_MODEL'),
  ]);

  if (currentProvider && currentModel) {
    return;
  }

  const fallback = getDesktopFallbackModelAndProvider();
  const nextProvider = currentProvider || fallback.provider;
  const nextModel =
    currentModel || (currentProvider && currentProvider !== fallback.provider ? '' : fallback.model);

  if (!nextProvider || !nextModel) {
    return;
  }

  await setConfigProvider({
    body: {
      provider: nextProvider,
      model: nextModel,
    },
    throwOnError: true,
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
  } = {
    working_dir: workingDir,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe = await decodeRecipe(options.recipeDeeplink);
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

  await ensureSessionProviderAndModelConfigured();

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
