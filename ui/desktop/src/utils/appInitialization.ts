import { ChatType } from '../types/chat';
import { Recipe } from '../recipe';
import { initializeSystem } from './providerUtils';
import { initializeCostDatabase } from './costDatabase';
import {
  type ExtensionConfig,
  type FixedExtensionEntry,
  MalformedConfigError,
} from '../components/ConfigContext';
import { backupConfig, initConfig, readAllConfig, recoverConfig, validateConfig } from '../api';
import { COST_TRACKING_ENABLED } from '../updates';

interface InitializationDependencies {
  getExtensions?: (b: boolean) => Promise<FixedExtensionEntry[]>;
  addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  setPairChat: (chat: ChatType | ((prev: ChatType) => ChatType)) => void;
  provider: string;
  model: string;
}

export const initializeApp = async ({
  getExtensions,
  addExtension,
  setPairChat,
  provider,
  model,
}: InitializationDependencies) => {
  console.log(`Initializing app`);

  const urlParams = new URLSearchParams(window.location.search);
  const viewType = urlParams.get('view');
  const resumeSessionId = urlParams.get('resumeSessionId');
  const recipeConfig = window.appConfig.get('recipe');

  if (resumeSessionId) {
    console.log('Session resume detected, letting useChat hook handle navigation');
    await initializeForSessionResume({ getExtensions, addExtension, provider, model });
    return;
  }

  if (recipeConfig && typeof recipeConfig === 'object') {
    console.log('Recipe deeplink detected, initializing system for recipe');
    await initializeForRecipe({
      recipeConfig: recipeConfig as Recipe,
      getExtensions,
      addExtension,
      setPairChat,
      provider,
      model,
    });
    return;
  }

  if (viewType) {
    handleViewTypeDeepLink(viewType, recipeConfig);
    return;
  }

  const costDbPromise = COST_TRACKING_ENABLED
    ? initializeCostDatabase().catch((error) => {
        console.error('Failed to initialize cost database:', error);
      })
    : (() => {
        console.log('Cost tracking disabled, skipping cost database initialization');
        return Promise.resolve();
      })();

  await initConfig();

  try {
    await readAllConfig({ throwOnError: true });
  } catch (error) {
    console.warn('Initial config read failed, attempting recovery:', error);
    await handleConfigRecovery();
  }

  if (provider && model) {
    try {
      const initPromises = [
        initializeSystem(provider, model, {
          getExtensions,
          addExtension,
        }),
      ];

      if (COST_TRACKING_ENABLED) {
        initPromises.push(costDbPromise);
      }

      await Promise.all(initPromises);
    } catch (error) {
      console.error('Error in system initialization:', error);
      if (error instanceof MalformedConfigError) {
        throw error;
      }
    }
  }
  window.location.hash = '#/';
  window.history.replaceState({}, '', '#/');
};

const initializeForSessionResume = async ({
  getExtensions,
  addExtension,
  provider,
  model,
}: Pick<InitializationDependencies, 'getExtensions' | 'addExtension' | 'provider' | 'model'>) => {
  await initConfig();
  await readAllConfig({ throwOnError: true });

  await initializeSystem(provider, model, {
    getExtensions,
    addExtension,
  });
};

const initializeForRecipe = async ({
  recipeConfig,
  getExtensions,
  addExtension,
  setPairChat,
  provider,
  model,
}: Pick<
  InitializationDependencies,
  'getExtensions' | 'addExtension' | 'setPairChat' | 'provider' | 'model'
> & {
  recipeConfig: Recipe;
}) => {
  await initConfig();
  await readAllConfig({ throwOnError: true });

  await initializeSystem(provider, model, {
    getExtensions,
    addExtension,
  });

  setPairChat((prevChat) => ({
    ...prevChat,
    recipeConfig: recipeConfig,
    title: recipeConfig?.title || 'Recipe Chat',
    messages: [],
    messageHistoryIndex: 0,
  }));

  window.location.hash = '#/pair';
  window.history.replaceState(
    {
      recipeConfig: recipeConfig,
      resetChat: true,
    },
    '',
    '#/pair'
  );
};

const handleViewTypeDeepLink = (viewType: string, recipeConfig: unknown) => {
  if (viewType === 'recipeEditor' && recipeConfig) {
    window.location.hash = '#/recipe-editor';
    window.history.replaceState({ config: recipeConfig }, '', '#/recipe-editor');
  } else {
    const routeMap: Record<string, string> = {
      chat: '#/',
      pair: '#/pair',
      settings: '#/settings',
      sessions: '#/sessions',
      schedules: '#/schedules',
      recipes: '#/recipes',
      permission: '#/permission',
      ConfigureProviders: '#/configure-providers',
      sharedSession: '#/shared-session',
      recipeEditor: '#/recipe-editor',
      welcome: '#/welcome',
    };

    const route = routeMap[viewType];
    if (route) {
      window.location.hash = route;
      window.history.replaceState({}, '', route);
    }
  }
};

const handleConfigRecovery = async () => {
  const configVersion = localStorage.getItem('configVersion');
  const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;

  if (shouldMigrateExtensions) {
    console.log('Performing extension migration...');
    try {
      await backupConfig({ throwOnError: true });
      await initConfig();
    } catch (migrationError) {
      console.error('Migration failed:', migrationError);
    }
  }

  console.log('Attempting config recovery...');
  try {
    await validateConfig({ throwOnError: true });
    await readAllConfig({ throwOnError: true });
  } catch {
    console.log('Config validation failed, attempting recovery...');
    try {
      await recoverConfig({ throwOnError: true });
      await readAllConfig({ throwOnError: true });
    } catch {
      console.warn('Config recovery failed, reinitializing...');
      await initConfig();
    }
  }
};
