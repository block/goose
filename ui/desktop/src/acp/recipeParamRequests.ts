import type {
  RecipeParameterDto,
  RecipeParamsResponse_unstable,
  RequestRecipeParams_unstable,
} from '@aaif/goose-sdk';
import { v7 as uuidv7 } from 'uuid';
import { USE_ACP_CHAT } from '../acpChatFeatureFlag';

export interface AcpRecipeParamRequest {
  id: string;
  sessionId: string;
  parameters: RecipeParameterDto[];
  initialValues?: Record<string, string>;
}

interface PendingRecipeParamRequest {
  request: AcpRecipeParamRequest;
  resolve: (response: RecipeParamsResponse_unstable) => void;
}

const pendingRequests = new Map<string, PendingRecipeParamRequest>();
const listeners = new Set<() => void>();
let snapshot: AcpRecipeParamRequest[] = [];

function emit(): void {
  snapshot = Array.from(pendingRequests.values(), (pending) => pending.request);
  for (const listener of listeners) {
    listener();
  }
}

export function subscribeAcpRecipeParamRequests(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getAcpRecipeParamRequestsSnapshot(): AcpRecipeParamRequest[] {
  return snapshot;
}

function configuredParameterValues(): Record<string, string> {
  const configured = window.appConfig?.get('recipeParameters') as
    | Record<string, string>
    | undefined;
  return configured ?? {};
}

export async function requestAcpRecipeParams(
  request: RequestRecipeParams_unstable
): Promise<RecipeParamsResponse_unstable> {
  if (!USE_ACP_CHAT) {
    return { action: 'cancel' };
  }

  const initialValues = configuredParameterValues();
  const paramRequest: AcpRecipeParamRequest = {
    id: `acp_recipe_params_${uuidv7()}`,
    sessionId: request.sessionId,
    parameters: request.parameters,
    initialValues,
  };

  return new Promise<RecipeParamsResponse_unstable>((resolve) => {
    pendingRequests.set(paramRequest.id, { request: paramRequest, resolve });
    emit();
  });
}

export function resolveAcpRecipeParamRequest(id: string, values: Record<string, string>): boolean {
  const pending = pendingRequests.get(id);
  if (!pending) {
    return false;
  }
  pendingRequests.delete(id);
  emit();
  pending.resolve({ action: 'submit', values });
  return true;
}

export function cancelAcpRecipeParamRequest(id: string): void {
  const pending = pendingRequests.get(id);
  if (!pending) {
    return;
  }
  pendingRequests.delete(id);
  emit();
  pending.resolve({ action: 'cancel' });
}
