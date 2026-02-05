/**
 * Utilities for fetching canonical model information from the backend
 */

import { getCanonicalModelInfo, type ModelInfoData } from '../api';

/**
 * Fetch canonical model info (pricing + context limits) for a specific provider/model
 */
export async function fetchCanonicalModelInfo(
  provider: string,
  model: string
): Promise<ModelInfoData | null> {
  try {
    const response = await getCanonicalModelInfo({
      body: { provider, model },
      throwOnError: true,
    });

    return response.data.model_info;
  } catch {
    // 404 means model not found in canonical registry
    return null;
  }
}
