import { NavigateFunction } from 'react-router-dom';

export type View =
  | 'welcome'
  | 'chat'
  | 'pair'
  | 'settings'
  | 'extensions'
  | 'moreModels'
  | 'configureProviders'
  | 'configPage'
  | 'ConfigureProviders'
  | 'settingsV2'
  | 'sessions'
  | 'schedules'
  | 'sharedSession'
  | 'loading'
  | 'recipeEditor'
  | 'recipes'
  | 'permission'
  | 'apps';

// TODO(Douwe): check these for usage, especially key: string for resetChat
export type ViewOptions = {
  extensionId?: string;
  showEnvVars?: boolean;
  deepLinkConfig?: unknown;
  sessionDetails?: unknown;
  error?: string;
  baseUrl?: string;
  config?: unknown;
  parentView?: View;
  parentViewOptions?: ViewOptions;
  disableAnimation?: boolean;
  initialMessage?: string;
  resetChat?: boolean;
  shareToken?: string;
};

export const createNavigationHandler = (navigate: NavigateFunction) => {
  return (view: View, options?: ViewOptions) => {
    switch (view) {
      case 'chat':
        navigate('/', { state: options });
        break;
      case 'pair':
        navigate('/pair', { state: options });
        break;
      case 'settings':
        navigate('/settings', { state: options });
        break;
      case 'sessions':
        navigate('/sessions', { state: options });
        break;
      case 'schedules':
        navigate('/schedules', { state: options });
        break;
      case 'recipes':
        navigate('/recipes', { state: options });
        break;
      case 'permission':
        navigate('/permission', { state: options });
        break;
      case 'ConfigureProviders':
        navigate('/configure-providers', { state: options });
        break;
      case 'sharedSession':
        navigate('/shared-session', { state: options });
        break;
      case 'recipeEditor':
        navigate('/recipe-editor', { state: options });
        break;
      case 'welcome':
        navigate('/welcome', { state: options });
        break;
      case 'extensions':
        navigate('/extensions', { state: options });
        break;
      case 'apps':
        navigate('/apps');
        break;
      default:
        navigate('/', { state: options });
    }
  };
};
