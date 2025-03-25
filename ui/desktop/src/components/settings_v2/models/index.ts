import { getApiUrl, getSecretKey } from '@/src/config';
import { initializeAgent } from '../../../agent/index';
import { ToastError, ToastLoading, ToastSuccess } from '../../settings/models/toasts';

// titles
const CHANGE_MODEL_TOAST_TITLE = 'Model selected';
const START_AGENT_TITLE = 'Initialize agent';

// errors
const SWITCH_MODEL_AGENT_ERROR_MSG = 'Failed to start agent with selected model';
const CONFIG_UPDATE_ERROR_MSG = 'Failed to update configuration settings';
const CONFIG_READ_MODEL_ERROR_MSG = 'Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config';

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
    // show toast with error
    ToastError({
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
    // show toast with error
    ToastError({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: CONFIG_UPDATE_ERROR_MSG,
      traceback: error,
    });
    // agent and config will be out of sync at this point
    // TODO: reset agent to use current config settings
  } finally {
    // show toast
    ToastSuccess({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: `${SWITCH_MODEL_SUCCESS_MSG} -- using ${model} from ${provider}`,
    });
  }
}

interface startAgentFromConfigProps {
  readFromConfig: (key: string, is_secret: boolean) => Promise<string>;
}

// starts agent with the values for GOOSE_PROVIDER and GOOSE_MODEL that are in the config
export async function startAgentFromConfig({ readFromConfig }: startAgentFromConfigProps) {
  let model: string;
  let provider: string;

  // read from config
  try {
    model = (await readFromConfig('GOOSE_MODEL', false)) as string;
    provider = (await readFromConfig('GOOSE_PROVIDER', false)) as string;
  } catch (error) {
    console.error(`Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config`);
    // show toast with error
    ToastError({
      title: START_AGENT_TITLE,
      msg: CONFIG_READ_MODEL_ERROR_MSG,
      traceback: error,
    });
    return;
  }

  console.log(`Starting agent with GOOSE_MODEL=${model} and GOOSE_PROVIDER=${provider}`);

  try {
    await initializeAgent({ model: model, provider: provider });
  } catch (error) {
    console.error(`Failed to change model at agent step -- ${model} ${provider}`);
    // show toast with error
    ToastError({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: SWITCH_MODEL_AGENT_ERROR_MSG,
      traceback: error,
    });
    return;
  } finally {
    // success toast
    ToastSuccess({
      title: CHANGE_MODEL_TOAST_TITLE,
      msg: `${INITIALIZE_SYSTEM_WITH_MODEL_SUCCESS_MSG} with ${model} from ${provider}`,
    });
  }
}
