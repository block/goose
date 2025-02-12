import { SuperRoutesConfigManagementService } from './generated';
import { getApiUrl, getSecretKey } from '../config';

// Initialize OpenAPI configuration
import { OpenAPI } from './generated/core/OpenAPI';
OpenAPI.BASE = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT');
OpenAPI.HEADERS = {
  'Content-Type': 'application/json',
  'X-Secret-Key': window.appConfig.get('secretKey'),
};

export class Config {
  static async upsert(key: string, value: any, isSecret?: boolean) {
    return await SuperRoutesConfigManagementService.upsertConfig({
      key,
      value,
      is_secret: isSecret,
    });
  }

  static async read(key: string) {
    return await SuperRoutesConfigManagementService.readConfig({ key });
  }

  static async remove(key: string) {
    return await SuperRoutesConfigManagementService.removeConfig({ key });
  }

  static async readAll() {
    const response = await SuperRoutesConfigManagementService.readAllConfig();
    return response.config;
  }

  static async addExtension(name: string, config: any) {
    return await SuperRoutesConfigManagementService.addExtension({ name, config });
  }

  static async removeExtension(name: string) {
    return await SuperRoutesConfigManagementService.removeExtension({ key: name });
  }
}
