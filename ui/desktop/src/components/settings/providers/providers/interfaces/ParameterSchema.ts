export default interface ParameterSchema {
  name: string;
  is_secret: boolean;
  location?: string; // env, config.yaml, and/or keychain
}
