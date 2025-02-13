import {
  readAllConfig,
  readConfig,
  removeConfig,
  upsertConfig,
  addExtension,
  removeExtension,
} from './generated';
import { client } from './generated/client.gen';

// Initialize client configuration
client.setConfig({
  baseUrl: window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT'),
  headers: {
    'Content-Type': 'application/json',
    'X-Secret-Key': window.appConfig.get('secretKey'),
  },
});

export class Config {
  static async upsert(key: string, value: any, isSecret?: boolean) {
    return await upsertConfig({
      body: {
        key,
        value,
        is_secret: isSecret,
      },
    });
  }

  static async read(key: string) {
    return await readConfig({
      body: { key },
    });
  }

  static async remove(key: string) {
    return await removeConfig({
      body: { key },
    });
  }

  static async readAll() {
    const response = await readAllConfig();
    return response.data.config;
  }

  static async addExtension(name: string, config: any) {
    return await addExtension({
      body: { name, config },
    });
  }

  static async removeExtension(name: string) {
    return await removeExtension({
      body: { key: name },
    });
  }
}
