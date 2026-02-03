"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";
import {
  getAllConnectedAccounts,
  deleteConnectedAccount,
  updateConnectedAccount,
  getOauth2LinkURL,
  createConnectedAccount,
  CreateConnectedAccountRequest,
  OAuth2ConnectedAccountResponse,
} from "@/features/connected-accounts/api/connectedaccount";
import { useMetaInfo } from "@/components/context/metainfo";
import { ConnectedAccount } from "@/features/connected-accounts/types/connectedaccount.types";
import { toast } from "sonner";

export const connectedAccountKeys = {
  all: () => ["connectedaccounts"] as const,
  filtered: (configIds?: string[]) => ["connectedaccounts", { configIds }] as const,
};

export const useConnectedAccounts = (configIds?: string[]) => {
  const { accessToken } = useMetaInfo();

  return useQuery<ConnectedAccount[], Error>({
    queryKey: configIds ? connectedAccountKeys.filtered(configIds) : connectedAccountKeys.all(),
    queryFn: () => getAllConnectedAccounts(accessToken!, configIds),
    enabled: !!accessToken,
  });
};

export const useAppConnectedAccounts = (appName?: string | null) => {
  const base = useConnectedAccounts();
  return {
    ...base,
    data: useMemo(
      () =>
        appName && base.data
          ? base.data.filter((a) => a.mcp_server_configuration?.mcp_server?.name === appName)
          : [],
      [base.data, appName],
    ),
  };
};

// Generic hook for creating connected accounts of any auth type
type CreateConnectedAccountParams = {
  mcpServerConfigurationId: string;
  apiKey?: string; // For API_KEY auth type
  redirectUrl?: string; // For OAUTH2 auth type
};

export const useCreateConnectedAccount = () => {
  const queryClient = useQueryClient();
  const { accessToken } = useMetaInfo();

  return useMutation<
    OAuth2ConnectedAccountResponse | ConnectedAccount,
    Error,
    CreateConnectedAccountParams
  >({
    mutationFn: (params) => {
      const request: CreateConnectedAccountRequest = {
        mcp_server_configuration_id: params.mcpServerConfigurationId,
        api_key: params.apiKey,
        redirect_url_after_account_creation: params.redirectUrl,
      };
      return createConnectedAccount(request, accessToken!);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: connectedAccountKeys.all(),
      });
    },
    onError: (error) => {
      toast.error(error.message);
    },
  });
};
type GetOauth2LinkURLParams = {
  appName: string;
  connectedAccountOwnerId: string;
  afterOAuth2LinkRedirectURL?: string;
};

export const useGetOauth2LinkURL = () => {
  return useMutation<string, Error, GetOauth2LinkURLParams>({
    mutationFn: (params) =>
      getOauth2LinkURL(
        params.appName,
        params.connectedAccountOwnerId,
        params.afterOAuth2LinkRedirectURL,
      ),
    onError: (error) => {
      toast.error(error.message);
    },
  });
};

type DeleteConnectedAccountParams = {
  connectedAccountId: string;
};

export const useDeleteConnectedAccount = () => {
  const queryClient = useQueryClient();
  const { accessToken } = useMetaInfo();

  return useMutation<void, Error, DeleteConnectedAccountParams>({
    mutationFn: (params) => deleteConnectedAccount(params.connectedAccountId, accessToken!),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: connectedAccountKeys.all(),
      }),
  });
};

type UpdateConnectedAccountParams = {
  connectedAccountId: string;
  enabled: boolean;
};

export const useUpdateConnectedAccount = () => {
  const queryClient = useQueryClient();

  return useMutation<ConnectedAccount, Error, UpdateConnectedAccountParams>({
    mutationFn: (params) => updateConnectedAccount(params.connectedAccountId, params.enabled),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: connectedAccountKeys.all(),
      }),
    onError: (error) => {
      toast.error(error.message);
    },
  });
};
