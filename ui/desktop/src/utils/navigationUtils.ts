import type { NavigateFunction } from 'react-router-dom';
import type { Recipe } from '../api';
import type { UserInput } from '../types/message';

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
  | 'recipes'
  | 'agents'
  | 'analytics'
  | 'monitoring'
  | 'evaluate'
  | 'permission';

export type ViewOptions = {
  showEnvVars?: boolean;
  deepLinkConfig?: unknown;
  sessionDetails?: unknown;
  error?: string;
  baseUrl?: string;
  recipe?: Recipe;
  parentView?: View;
  parentViewOptions?: ViewOptions;
  disableAnimation?: boolean;
  initialMessage?: UserInput;
  shareToken?: string;
  resumeSessionId?: string;
  pendingScheduleDeepLink?: string;
};

export const createNavigationHandler = (navigate: NavigateFunction) => {
  return (view: View, options?: ViewOptions) => {
    switch (view) {
      case 'chat':
        navigate('/', { state: options });
        break;
      case 'pair': {
        // Put resumeSessionId in URL search params (not just state) so that:
        // 1. The sidebar can read it to highlight the active session
        // 2. Page refresh preserves which session is active
        // 3. Browser back/forward navigation works correctly
        const searchParams = new URLSearchParams();
        if (options?.resumeSessionId) {
          searchParams.set('resumeSessionId', options.resumeSessionId);
        }
        const url = searchParams.toString() ? `/pair?${searchParams.toString()}` : '/pair';
        navigate(url, { state: options });
        break;
      }
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
      case 'agents':
        navigate('/agents', { state: options });
        break;
      case 'analytics':
        navigate('/monitoring', { state: options });
        break;
      case 'monitoring':
        navigate('/monitoring', { state: options });
        break;
      case 'evaluate':
        navigate('/evaluate', { state: options });
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

      case 'welcome':
        navigate('/welcome', { state: options });
        break;
      case 'extensions':
        navigate('/extensions', { state: options });
        break;
      default:
        navigate('/', { state: options });
    }
  };
};
