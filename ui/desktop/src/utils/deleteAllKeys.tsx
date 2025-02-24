import { getApiUrl, getSecretKey } from '../config';
import { required_keys } from '../components/settings/models/hardcoded_stuff';

export async function DeleteProviderKeysFromKeychain() {
  for (const [provider, keys] of Object.entries(required_keys)) {
    for (const keyName of keys) {
      try {
        const deleteResponse = await fetch(getApiUrl('/configs/delete'), {
          method: 'DELETE',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({
            key: keyName,
            is_secret: true, // get rid of keychain keys only
          }),
        });

        if (!deleteResponse.ok) {
          const errorText = await deleteResponse.text();
          console.error('Delete response error:', errorText);
          throw new Error('Failed to delete key: ' + keyName);
        } else {
          console.log('Successfully deleted key:', keyName);
        }
      } catch (error) {
        console.error('Error deleting key:', keyName, error);
      }
    }
  }
}
