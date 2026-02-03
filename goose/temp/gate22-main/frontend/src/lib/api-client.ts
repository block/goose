import { useQuery, useMutation, UseQueryOptions, UseMutationOptions } from "@tanstack/react-query";
import { tokenManager } from "./token-manager";
import { throwApiError } from "./api-error-handler";
import { OrganizationRole } from "@/features/settings/types/organization.types";

// Get API base URL from environment
export const getApiBaseUrl = () => {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";
};

// Get MCP base URL from environment
export const getMcpBaseUrl = () => {
  return (
    process.env.NEXT_PUBLIC_MCP_BASE_URL ||
    process.env.NEXT_PUBLIC_API_URL ||
    "http://localhost:8000"
  );
};

// Generic fetcher function with automatic token refresh
export async function fetcher<T = unknown>(
  endpoint: string,
  options?: RequestInit & { params?: Record<string, string> },
): Promise<T> {
  const baseUrl = getApiBaseUrl();
  let url = `${baseUrl}${endpoint}`;

  // Add query parameters if provided
  if (options?.params) {
    const searchParams = new URLSearchParams(options.params);
    url += `?${searchParams.toString()}`;
  }

  const makeRequest = async (token?: string) => {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...(options?.headers as Record<string, string>),
    };

    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    return fetch(url, {
      ...options,
      headers,
    });
  };

  let response = await makeRequest();

  // If unauthorized and we haven't provided a token, try to get one
  const headersRecord = options?.headers as Record<string, string> | undefined;
  if (response.status === 401 && !headersRecord?.["Authorization"]) {
    try {
      const token = await tokenManager.getAccessToken();
      if (token) {
        response = await makeRequest(token);
      }
    } catch {
      // Token refresh failed, proceed with original response
    }
  }

  if (!response.ok) {
    await throwApiError(response, `Request failed: ${response.status}`);
  }

  // Handle empty responses (204 No Content)
  if (response.status === 204) {
    return {} as T;
  }

  return response.json();
}

// Fetcher with authentication and automatic token refresh
export function fetcherWithAuth<T = unknown>(
  token?: string,
  organizationId?: string,
  userRole?: string,
) {
  return async (
    endpoint: string,
    options?: RequestInit & { params?: Record<string, string> },
  ): Promise<T> => {
    // Use provided token or get from token manager (which handles refresh automatically)
    // Pass organization context for role-aware token generation
    const validToken =
      token || (await tokenManager.getAccessToken(organizationId, userRole as OrganizationRole));

    if (!validToken) {
      throw new Error("No valid authentication token");
    }

    const baseUrl = getApiBaseUrl();
    let url = `${baseUrl}${endpoint}`;

    // Add query parameters if provided
    if (options?.params) {
      const searchParams = new URLSearchParams(options.params);
      url += `?${searchParams.toString()}`;
    }

    const makeRequest = async (authToken: string) => {
      return fetch(url, {
        ...options,
        headers: {
          "Content-Type": "application/json",
          ...options?.headers,
          Authorization: `Bearer ${authToken}`,
        },
      });
    };

    let response = await makeRequest(validToken);

    // If unauthorized, the token might have expired between getting it and using it
    // Try once more with a fresh token
    if (response.status === 401) {
      tokenManager.clearToken(); // Clear the stale token
      const freshToken = await tokenManager.getAccessToken(
        organizationId,
        userRole as OrganizationRole,
      );
      if (freshToken) {
        response = await makeRequest(freshToken);
      }
    }

    if (!response.ok) {
      await throwApiError(response, `Request failed: ${response.status}`);
    }

    // Handle empty responses (204 No Content)
    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  };
}

// Custom hook for API queries
export function useApiQuery<T = unknown>(
  key: string | string[],
  endpoint: string,
  token?: string,
  options?: Omit<UseQueryOptions<T>, "queryKey" | "queryFn"> & {
    organizationId?: string;
    userRole?: string;
  },
) {
  const queryKey = Array.isArray(key) ? key : [key];
  const { organizationId, userRole, ...queryOptions } = options || {};

  return useQuery<T>({
    queryKey,
    queryFn: () => {
      if (token) {
        return fetcherWithAuth<T>(token, organizationId, userRole)(endpoint);
      }
      return fetcher<T>(endpoint);
    },
    ...queryOptions,
  });
}

// Custom hook for API mutations
export function useApiMutation<TData = unknown, TVariables = unknown>(
  mutationFn: (variables: TVariables) => Promise<TData>,
  options?: UseMutationOptions<TData, Error, TVariables>,
) {
  return useMutation<TData, Error, TVariables>({
    mutationFn,
    ...options,
  });
}

// Helper to create authenticated API calls
export const createAuthenticatedRequest = (
  token?: string,
  organizationId?: string,
  userRole?: string,
) => ({
  get: <T = unknown>(endpoint: string, params?: Record<string, string>) =>
    fetcherWithAuth<T>(token, organizationId, userRole)(endpoint, { method: "GET", params }),

  post: <T = unknown>(endpoint: string, data?: unknown) =>
    fetcherWithAuth<T>(
      token,
      organizationId,
      userRole,
    )(endpoint, {
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    }),

  patch: <T = unknown>(endpoint: string, data?: unknown) =>
    fetcherWithAuth<T>(
      token,
      organizationId,
      userRole,
    )(endpoint, {
      method: "PATCH",
      body: data ? JSON.stringify(data) : undefined,
    }),

  put: <T = unknown>(endpoint: string, data?: unknown) =>
    fetcherWithAuth<T>(
      token,
      organizationId,
      userRole,
    )(endpoint, {
      method: "PUT",
      body: data ? JSON.stringify(data) : undefined,
    }),

  delete: <T = unknown>(endpoint: string) =>
    fetcherWithAuth<T>(token, organizationId, userRole)(endpoint, { method: "DELETE" }),
});
