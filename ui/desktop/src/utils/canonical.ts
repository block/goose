/**
 * Utilities for fetching canonical model information from the backend
 */

import { acpGetCanonicalModelInfo, type CanonicalModelInfoDto } from '../acp/providers';

export type CanonicalModelInfo = CanonicalModelInfoDto;

/**
 * Fetch canonical model info (pricing + context limits) for a specific provider/model
 */
export async function fetchCanonicalModelInfo(
  provider: string,
  model: string
): Promise<CanonicalModelInfoDto | null> {
  try {
    return await acpGetCanonicalModelInfo(provider, model);
  } catch {
    return null;
  }
}
