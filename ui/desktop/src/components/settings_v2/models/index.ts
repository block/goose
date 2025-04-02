import { initializeSystem } from '../../../utils/providerUtils';
import { toastError, toastSuccess } from '../../../toasts';
import { ProviderDetails } from '@/src/api';
import Model, { getProviderMetadata } from './modelInterface';
import { ProviderMetadata } from '../../../api';
import type { ExtensionConfig, FixedExtensionEntry } from '../../ConfigContext';

// titles
export const UNKNOWN_PROVIDER_TITLE = 'Provider name lookup';

// errors
const CHANGE_MODEL_ERROR_TITLE = 'Change failed';
const SWITCH_MODEL_AGENT_ERROR_MSG =
  'Failed to start agent with selected model -- please try again';
const CONFIG_UPDATE_ERROR_MSG = 'Failed to update configuration settings -- please try again';
export const UNKNOWN_PROVIDER_MSG = 'Unknown provider in config -- please inspect your config.yaml';

// success
const CHANGE_MODEL_TOAST_TITLE = 'Model changed';
const SWITCH_MODEL_SUCCESS_MSG = 'Successfully switched models';

interface changeModelProps {
  model: Model;
  writeToConfig: (key: string, value: unknown, is_secret: boolean) => Promise<void>;
  getExtensions?: (b: boolean) => Promise<FixedExtensionEntry[]>;
  addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
}

// TODO: error handling
export async function changeModel({
  model,
  writeToConfig,
  getExtensions,
  addExtension,
}: changeModelProps) {
  const modelName = model.name;
  const providerName = model.provider;
  try {
    await initializeSystem(providerName, modelName, {
      getExtensions,
      addExtension,
    });
  } catch (error) {
    console.error(`Failed to change model at agent step -- ${modelName} ${providerName}`);
    toastError({
      title: CHANGE_MODEL_ERROR_TITLE,
      msg: SWITCH_MODEL_AGENT_ERROR_MSG,
      traceback: error,
    });
    // don't write to config
    return;
  }

  try {
    await writeToConfig('GOOSE_PROVIDER', providerName, false);
    await writeToConfig('GOOSE_MODEL', modelName, false);
  } catch (error) {
    console.error(`Failed to change model at config step -- ${modelName} ${providerName}}`);
    toastError({
      title: CHANGE_MODEL_ERROR_TITLE,
      msg: CONFIG_UPDATE_ERROR_MSG,
      traceback: error,
    });
    // agent and config will be out of sync at this point
    // TODO: reset agent to use current config settings
  } finally {
    // show toast
    toastSuccess({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: `${SWITCH_MODEL_SUCCESS_MSG} -- using ${model.alias ?? modelName} from ${model.subtext ?? providerName}`,
    });
  }
}

interface getCurrentModelAndProviderProps {
  readFromConfig: (key: string, is_secret: boolean) => Promise<unknown>;
}

export async function getCurrentModelAndProvider({
  readFromConfig,
}: getCurrentModelAndProviderProps) {
  let model: string;
  let provider: string;

  // read from config
  try {
    model = (await readFromConfig('GOOSE_MODEL', false)) as string;
    provider = (await readFromConfig('GOOSE_PROVIDER', false)) as string;
  } catch (error) {
    console.error(`Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config`);
    throw error;
  }
  return { model: model, provider: provider };
}

interface getCurrentModelAndProviderForDisplayProps {
  readFromConfig: (key: string, is_secret: boolean) => Promise<unknown>;
  getProviders: (b: boolean) => Promise<ProviderDetails[]>;
}

// returns display name of the provider
export async function getCurrentModelAndProviderForDisplay({
  readFromConfig,
  getProviders,
}: getCurrentModelAndProviderForDisplayProps) {
  const modelProvider = await getCurrentModelAndProvider({ readFromConfig: readFromConfig });
  const gooseModel = modelProvider.model;
  const gooseProvider = modelProvider.provider;

  // lookup display name
  let metadata: ProviderMetadata;

  try {
    metadata = await getProviderMetadata(gooseProvider, getProviders);
  } catch (error) {
    return { model: gooseModel, provider: gooseProvider };
  }
  const providerDisplayName = metadata.display_name;

  return { model: gooseModel, provider: providerDisplayName };
}
