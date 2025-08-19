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
  read: (key: string, is_secret: boolean) => Promise<unknown>;
  setPairChat: (chat: ChatType | ((prev: ChatType) => ChatType)) => void;
  setFatalError: (error: string) => void;
}

export const performAppInitialization = async ({
  getExtensions,
  addExtension,
  read,
  setPairChat,
  setFatalError,
}: InitializationDependencies) => {
  console.log(`Initializing app`);

  const urlParams = new URLSearchParams(window.location.search);
  const viewType = urlParams.get('view');
  const resumeSessionId = urlParams.get('resumeSessionId');
  const recipeConfig = window.appConfig.get('recipe');

  // Check for session resume first - this takes priority over other navigation
  if (resumeSessionId) {
    console.log('Session resume detected, letting useChat hook handle navigation');
    await initializeForSessionResume({ getExtensions, addExtension, read, setFatalError });
    return;
  }

  // Check for recipe config - this also needs provider initialization
  if (recipeConfig && typeof recipeConfig === 'object') {
    console.log('Recipe deeplink detected, initializing system for recipe');
    await initializeForRecipe({
      recipeConfig: recipeConfig as Recipe,
      getExtensions,
      addExtension,
      read,
      setPairChat,
      setFatalError,
    });
    return;
  }

  if (viewType) {
    handleViewTypeDeepLink(viewType, recipeConfig);
    return;
  }

  await initializeApp({ getExtensions, addExtension, read, setFatalError });
};

const initializeForSessionResume = async ({
  getExtensions,
  addExtension,
  read,
  setFatalError,
}: Pick<
  InitializationDependencies,
  'getExtensions' | 'addExtension' | 'read' | 'setFatalError'
>) => {
  try {
    await initConfig();
    await readAllConfig({ throwOnError: true });

    const config = window.electron.getConfig();
    const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
    const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

    if (provider && model) {
      await initializeSystem(provider as string, model as string, {
        getExtensions,
        addExtension,
      });
    } else {
      throw new Error('No provider/model configured for session resume');
    }
  } catch (error) {
    console.error('Failed to initialize system for session resume:', error);
    setFatalError(
      `Failed to initialize system for session resume: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
};

const initializeForRecipe = async ({
  recipeConfig,
  getExtensions,
  addExtension,
  read,
  setPairChat,
  setFatalError,
}: Pick<
  InitializationDependencies,
  'getExtensions' | 'addExtension' | 'read' | 'setPairChat' | 'setFatalError'
> & {
  recipeConfig: Recipe;
}) => {
  try {
    await initConfig();
    await readAllConfig({ throwOnError: true });

    const config = window.electron.getConfig();
    const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
    const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

    if (provider && model) {
      await initializeSystem(provider as string, model as string, {
        getExtensions,
        addExtension,
      });

      // Set up the recipe in pair chat after system is initialized
      setPairChat((prevChat) => ({
        ...prevChat,
        recipeConfig: recipeConfig,
        title: recipeConfig?.title || 'Recipe Chat',
        messages: [], // Start fresh for recipe
        messageHistoryIndex: 0,
      }));

      // Navigate to pair view
      window.location.hash = '#/pair';
      window.history.replaceState(
        {
          recipeConfig: recipeConfig,
          resetChat: true,
        },
        '',
        '#/pair'
      );
    } else {
      throw new Error('No provider/model configured for recipe');
    }
  } catch (error) {
    console.error('Failed to initialize system for recipe:', error);
    setFatalError(
      `Failed to initialize system for recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
};

const handleViewTypeDeepLink = (viewType: string, recipeConfig: unknown) => {
  if (viewType === 'recipeEditor' && recipeConfig) {
    // Handle recipe editor deep link - use hash routing
    window.location.hash = '#/recipe-editor';
    window.history.replaceState({ config: recipeConfig }, '', '#/recipe-editor');
  } else {
    // Handle other deep links by redirecting to appropriate route
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

const initializeApp = async ({
  getExtensions,
  addExtension,
  read,
  setFatalError,
}: Pick<
  InitializationDependencies,
  'getExtensions' | 'addExtension' | 'read' | 'setFatalError'
>) => {
  try {
    // Start cost database initialization early (non-blocking) - only if cost tracking is enabled
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

    const config = window.electron.getConfig();
    const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
    const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

    if (provider && model) {
      try {
        // Initialize system in parallel with cost database (if enabled)
        const initPromises = [
          initializeSystem(provider as string, model as string, {
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
        window.location.hash = '#/';
        window.history.replaceState({}, '', '#/');
      }
    } else {
      window.location.hash = '#/';
      window.history.replaceState({}, '', '#/');
    }
  } catch (error) {
    console.error('Fatal error during initialization:', error);
    setFatalError(error instanceof Error ? error.message : 'Unknown error occurred');
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
      // Continue with recovery attempts
    }
  }

  // Try recovery if migration didn't work or wasn't needed
  console.log('Attempting config recovery...');
  try {
    // Try to validate first (faster than recovery)
    await validateConfig({ throwOnError: true });
    // If validation passes, try reading again
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
