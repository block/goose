const KEYCHAIN_PREFIX = 'mock_keychain_';

const mockKeychain = {
  async setKey(key: string, value: string): Promise<void> {
    console.log('MockKeychain: Setting key:', { key, hasValue: !!value });
    try {
      localStorage.setItem(KEYCHAIN_PREFIX + key, value);
      console.log('MockKeychain: Successfully stored key:', key);
    } catch (error) {
      console.error('MockKeychain: Failed to store key:', {
        key,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  },

  async getKey(key: string): Promise<string | null> {
    const value = localStorage.getItem(KEYCHAIN_PREFIX + key);
    console.log('MockKeychain: Retrieved key:', { key, hasValue: !!value });
    return value;
  },

  async hasKey(key: string): Promise<boolean> {
    const exists = localStorage.getItem(KEYCHAIN_PREFIX + key) !== null;
    console.log('MockKeychain: Checking key existence:', { key, exists });
    return exists;
  },

  async deleteKey(key: string): Promise<void> {
    console.log('MockKeychain: Deleting key:', key);
    localStorage.removeItem(KEYCHAIN_PREFIX + key);
  },
};

export default mockKeychain;
