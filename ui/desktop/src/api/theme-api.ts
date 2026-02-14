/**
 * Custom theme API functions
 * These will be replaced when the OpenAPI spec is regenerated
 */

import { client } from './client.gen';
import type { ThemeColorsDto } from './types.gen';

export interface SaveCustomThemeRequest {
  id: string;
  name: string;
  author: string;
  description: string;
  tags: string[];
  colors: ThemeColorsDto;
}

export interface SaveCustomThemeResponse {
  message: string;
}

export interface DeleteCustomThemeResponse {
  message: string;
}

export interface ActiveThemeResponse {
  theme_id: string | null;
}

/**
 * Save a custom theme preset
 */
export const saveCustomTheme = async (
  request: SaveCustomThemeRequest
): Promise<SaveCustomThemeResponse> => {
  const response = await client.post<SaveCustomThemeResponse, unknown, false>({
    url: '/theme/save-custom',
    body: request,
    headers: {
      'Content-Type': 'application/json',
    },
  });
  
  return response.data;
};

/**
 * Delete a custom theme preset by ID
 */
export const deleteCustomTheme = async (id: string): Promise<DeleteCustomThemeResponse> => {
  const response = await client.delete<DeleteCustomThemeResponse, unknown, false>({
    url: `/theme/saved/${id}`,
  });
  
  return response.data;
};

/**
 * Get the currently active theme ID
 */
export const getActiveTheme = async (): Promise<ActiveThemeResponse> => {
  const response = await client.get<ActiveThemeResponse, unknown, false>({
    url: '/theme/active',
  });
  
  return response.data;
};
