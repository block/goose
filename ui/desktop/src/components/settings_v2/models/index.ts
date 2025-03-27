import { initializeAgent } from '../../../agent/index';
import { toastError, toastSuccess } from '../../../toasts';
import { ProviderDetails } from '@/src/api';

// titles
const CHANGE_MODEL_TOAST_TITLE = 'Model selected';
const START_AGENT_TITLE = 'Initialize agent';
const UNKNOWN_PROVIDER_TITLE = 'Provider name lookup';

// errors
const SWITCH_MODEL_AGENT_ERROR_MSG = 'Failed to start agent with selected model';
const CONFIG_UPDATE_ERROR_MSG = 'Failed to update configuration settings';
const CONFIG_READ_MODEL_ERROR_MSG = 'Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config';
const UNKNOWN_PROVIDER_MSG = 'Unknown provider in config -- please inspect your config.yaml';

// success
const SWITCH_MODEL_SUCCESS_MSG = 'Successfully switched models';
const INITIALIZE_SYSTEM_WITH_MODEL_SUCCESS_MSG = 'Successfully started Goose';

interface changeModelProps {
  model: string;
  provider: string;
  writeToConfig: (key: string, value: unknown, is_secret: boolean) => Promise<void>;
}

// TODO: error handling
export async function changeModel({ model, provider, writeToConfig }: changeModelProps) {
  try {
    await initializeAgent({ model: model, provider: provider });
  } catch (error) {
    console.error(`Failed to change model at agent step -- ${model} ${provider}`);
    toastError({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: SWITCH_MODEL_AGENT_ERROR_MSG,
      traceback: error,
    });
    // don't write to config
    return;
  }

  try {
    await writeToConfig('GOOSE_PROVIDER', provider, false);
    await writeToConfig('GOOSE_MODEL', model, false);
  } catch (error) {
    console.error(`Failed to change model at config step -- ${model} ${provider}`);
    toastError({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: CONFIG_UPDATE_ERROR_MSG,
      traceback: error,
    });
    // agent and config will be out of sync at this point
    // TODO: reset agent to use current config settings
  } finally {
    // show toast
    toastSuccess({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: `${SWITCH_MODEL_SUCCESS_MSG} -- using ${model} from ${provider}`,
    });
  }
}

interface startAgentFromConfigProps {
  readFromConfig: (key: string, is_secret: boolean) => Promise<unknown>;
}

// starts agent with the values for GOOSE_PROVIDER and GOOSE_MODEL that are in the config
export async function startAgentFromConfig({ readFromConfig }: startAgentFromConfigProps) {
  let modelProvider: { model: string; provider: string };

  // read from config
  try {
    modelProvider = await getCurrentModelAndProvider({ readFromConfig: readFromConfig });
  } catch (error) {
    toastError({
      title: START_AGENT_TITLE,
      msg: CONFIG_READ_MODEL_ERROR_MSG,
      traceback: error,
    });
    return;
  }

  const model = modelProvider.model;
  const provider = modelProvider.provider;

  console.log(`Starting agent with GOOSE_MODEL=${model} and GOOSE_PROVIDER=${provider}`);

  try {
    await initializeAgent({ model: model, provider: provider });
  } catch (error) {
    console.error(`Failed to change model at agent step -- ${model} ${provider}`);
    toastError({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: SWITCH_MODEL_AGENT_ERROR_MSG,
      traceback: error,
    });
    return;
  } finally {
    toastSuccess({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: `${INITIALIZE_SYSTEM_WITH_MODEL_SUCCESS_MSG} with ${model} from ${provider}`,
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

  const providers = await getProviders(false);

  // lookup display name
  const providerDetailsList = providers.filter((provider) => provider.name === gooseProvider);

  if (providerDetailsList.length != 1) {
    toastError({
      title: UNKNOWN_PROVIDER_TITLE,
      msg: UNKNOWN_PROVIDER_MSG,
    });
    return { model: gooseModel, provider: gooseProvider };
  }
  const providerDisplayName = providerDetailsList[0].metadata.display_name;

  return { model: gooseModel, provider: providerDisplayName };
}
