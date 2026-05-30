import { disconnect, getStatus, setup as setupGooseBotApi } from '../api/sdk.gen';

export async function startGooseBotSetup(): Promise<{ installation_id: number }> {
  const { data } = await setupGooseBotApi({ throwOnError: true });
  if (!data?.installation_id) {
    throw new Error('setup returned no installation_id');
  }
  return data;
}

export async function fetchGooseBotInstallId(): Promise<number | null> {
  const { data } = await getStatus({ throwOnError: true });
  const id = data?.installation_id;
  return typeof id === 'number' && Number.isFinite(id) ? id : null;
}

export async function disconnectGooseBot(): Promise<void> {
  await disconnect({ throwOnError: true });
}
