import OpenlitConfig from '../config';
import { OPENLIT_URL } from '../constant';
import { VaultOptions } from '../types';

export default class Vault {
  static async getSecrets(options: VaultOptions) {
    const url = process.env.OPENLIT_URL || options.url || OPENLIT_URL;
    const apiKey = process.env.OPENLIT_API_KEY || options.apiKey || '';

    const metaProperties = {
      applicationName: OpenlitConfig.applicationName,
      environment: OpenlitConfig.environment,
    };

    try {
      const vaultResponse = await fetch(`${url}/api/vault/get-secrets`, {
        method: 'POST',
        body: JSON.stringify({
          key: options.key,
          tags: options.tags,
          metaProperties,
          source: 'ts-sdk',
        }),
        headers: {
          Authorization: `Bearer ${apiKey}`,
        },
      })
        .then(async (response: any) => {
          if (!response.ok) {
            return {
              err: `Openlit Error : HTTP error! Status: ${response.status}`,
            };
          }
          return response.json();
        })
        .then((resp: any) => {
          return resp;
        });

      const { res = {} } = vaultResponse;
      if (!!options.shouldSetEnv) {
        Object.entries(res).forEach(([key, value]: [string, unknown]) => {
          process.env[key] = value as string;
        });
      }

      return vaultResponse;
    } catch (e: any) {
      console.log(e);
      if (e && typeof e.toString === 'function') {
        return {
          err: `Openlit Error : ${e.toString()}`,
          data: null,
        };
      }

      return {
        err: `Openlit Error : ${e}`,
      };
    }
  }
}
