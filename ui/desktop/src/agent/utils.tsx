import { getApiUrl, getSecretKey } from '../config';

export async function initializeAgent(model: string, provider: string) {
  console.log('fetching...', provider, model);
  const response = await fetch(getApiUrl('/agent'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model: model,
    }),
  });
  return response;
}

export async function replaceWithShims(cmd: string) {
  const binaryPathMap: Record<string, string> = {
    goosed: await window.electron.getBinaryPath('goosed'),
    jbang: await window.electron.getBinaryPath('jbang'),
    npx: await window.electron.getBinaryPath('npx'),
    uvx: await window.electron.getBinaryPath('uvx'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    cmd = binaryPathMap[cmd];
  }

  return cmd;
}
