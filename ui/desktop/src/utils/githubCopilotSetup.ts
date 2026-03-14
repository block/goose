import { configureProviderOauth } from '../api';
import type { OauthResponse, DeviceCodeResponse } from '../api/types.gen';

export async function startGitHubCopilotSetup(): Promise<{
  success: boolean;
  data?: DeviceCodeResponse;
  message: string;
}> {
  try {
    const response = await configureProviderOauth({
      path: { name: 'github_copilot' },
      throwOnError: true,
    });

    // Handle the discriminated union response
    const data = response.data as OauthResponse;

    if ('userCode' in data && 'verificationUri' in data) {
      // Device code response - user needs to enter code manually
      return {
        success: true,
        data: data,
        message: 'Device code received, please enter code on GitHub',
      };
    } else {
      // Completed response
      return {
        success: true,
        message: 'GitHub Copilot setup completed',
      };
    }
  } catch (e) {
    return {
      success: false,
      message: `Failed to start GitHub Copilot setup: ${e}`,
    };
  }
}
