import PromptHub from './prompt-hub';
import Vault from './vault';

export default class BaseOpenlit {
  static getPrompts = PromptHub.getPrompts;
  static getSecrets = Vault.getSecrets;
}
