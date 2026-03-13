import { configureProviderOauth } from '../api';
import type { OauthResponse, OauthCompletedResponse, DeviceCodeResponse } from '../api/types.gen';

export async function startChatGptCodexSetup(): Promise<{ success: boolean; message: string }> {
  try {
    await configureProviderOauth({
      path: { name: 'chatgpt_codex' },
      throwOnError: true,
    });
    return { success: true, message: 'ChatGPT Codex setup completed' };
  } catch (e) {
    return {
      success: false,
      message: `Failed to start ChatGPT Codex setup: ${e}`,
    };
  }
}

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

// Helper function to check if response is a device code response
export function isDeviceCodeResponse(response: OauthResponse): response is DeviceCodeResponse {
  return 'userCode' in response && 'verificationUri' in response;
}

// Helper function to check if response is a completed response
export function isCompletedResponse(response: OauthResponse): response is OauthCompletedResponse {
  return 'message' in response && !('userCode' in response);
}
