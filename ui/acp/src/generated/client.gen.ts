// This file is auto-generated — do not edit manually.

export interface ExtMethodProvider {
  extMethod(
    method: string,
    params: Record<string, unknown>,
  ): Promise<Record<string, unknown>>;
}

import type {
  AddExtensionRequest,
  ClearSessionRequest,
  DeleteSessionRequest,
  ExportSessionRequest,
  ExportSessionResponse,
  GetExtensionsResponse,
  GetPromptInfoRequest,
  GetPromptInfoResponse,
  GetSessionRequest,
  GetSessionResponse,
  GetToolsRequest,
  GetToolsResponse,
  ImportSessionRequest,
  ImportSessionResponse,
  ListPromptsRequest,
  ListPromptsResponse,
  ListSessionsResponse,
  PlanPromptRequest,
  PlanPromptResponse,
  ProviderInfoRequest,
  ProviderInfoResponse,
  ReadResourceRequest,
  ReadResourceResponse,
  RemoveExtensionRequest,
  UpdateWorkingDirRequest,
} from './types.gen.js';
import {
  zExportSessionResponse,
  zGetExtensionsResponse,
  zGetPromptInfoResponse,
  zGetSessionResponse,
  zGetToolsResponse,
  zImportSessionResponse,
  zListPromptsResponse,
  zListSessionsResponse,
  zPlanPromptResponse,
  zProviderInfoResponse,
  zReadResourceResponse,
} from './zod.gen.js';

export class GooseExtClient {
  constructor(private conn: ExtMethodProvider) {}

  async extensionsAdd(params: AddExtensionRequest): Promise<void> {
    await this.conn.extMethod("_goose/extensions/add", params);
  }

  async extensionsRemove(params: RemoveExtensionRequest): Promise<void> {
    await this.conn.extMethod("_goose/extensions/remove", params);
  }

  async tools(params: GetToolsRequest): Promise<GetToolsResponse> {
    const raw = await this.conn.extMethod("_goose/tools", params);
    return zGetToolsResponse.parse(raw) as GetToolsResponse;
  }

  async resourceRead(
    params: ReadResourceRequest,
  ): Promise<ReadResourceResponse> {
    const raw = await this.conn.extMethod("_goose/resource/read", params);
    return zReadResourceResponse.parse(raw) as ReadResourceResponse;
  }

  async workingDirUpdate(params: UpdateWorkingDirRequest): Promise<void> {
    await this.conn.extMethod("_goose/working_dir/update", params);
  }

  async sessionList(): Promise<ListSessionsResponse> {
    const raw = await this.conn.extMethod("_goose/session/list", {});
    return zListSessionsResponse.parse(raw) as ListSessionsResponse;
  }

  async sessionGet(params: GetSessionRequest): Promise<GetSessionResponse> {
    const raw = await this.conn.extMethod("_goose/session/get", params);
    return zGetSessionResponse.parse(raw) as GetSessionResponse;
  }

  async sessionDelete(params: DeleteSessionRequest): Promise<void> {
    await this.conn.extMethod("_goose/session/delete", params);
  }

  async sessionExport(
    params: ExportSessionRequest,
  ): Promise<ExportSessionResponse> {
    const raw = await this.conn.extMethod("_goose/session/export", params);
    return zExportSessionResponse.parse(raw) as ExportSessionResponse;
  }

  async sessionImport(
    params: ImportSessionRequest,
  ): Promise<ImportSessionResponse> {
    const raw = await this.conn.extMethod("_goose/session/import", params);
    return zImportSessionResponse.parse(raw) as ImportSessionResponse;
  }

  async configExtensions(): Promise<GetExtensionsResponse> {
    const raw = await this.conn.extMethod("_goose/config/extensions", {});
    return zGetExtensionsResponse.parse(raw) as GetExtensionsResponse;
  }

  async configPrompts(
    params: ListPromptsRequest,
  ): Promise<ListPromptsResponse> {
    const raw = await this.conn.extMethod("_goose/config/prompts", params);
    return zListPromptsResponse.parse(raw) as ListPromptsResponse;
  }

  async configPromptInfo(
    params: GetPromptInfoRequest,
  ): Promise<GetPromptInfoResponse> {
    const raw = await this.conn.extMethod("_goose/config/prompt_info", params);
    return zGetPromptInfoResponse.parse(raw) as GetPromptInfoResponse;
  }

  async agentProviderInfo(
    params: ProviderInfoRequest,
  ): Promise<ProviderInfoResponse> {
    const raw = await this.conn.extMethod("_goose/agent/provider_info", params);
    return zProviderInfoResponse.parse(raw) as ProviderInfoResponse;
  }

  async agentPlanPrompt(
    params: PlanPromptRequest,
  ): Promise<PlanPromptResponse> {
    const raw = await this.conn.extMethod("_goose/agent/plan_prompt", params);
    return zPlanPromptResponse.parse(raw) as PlanPromptResponse;
  }

  async sessionClear(params: ClearSessionRequest): Promise<void> {
    await this.conn.extMethod("_goose/session/clear", params);
  }
}
