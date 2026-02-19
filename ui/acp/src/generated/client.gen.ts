// This file is auto-generated — do not edit manually.

export interface ExtMethodProvider {
  extMethod(
    method: string,
    params: Record<string, unknown>,
  ): Promise<Record<string, unknown>>;
}

import type {
  AddExtensionRequest,
  DeleteSessionRequest,
  ExportSessionRequest,
  ExportSessionResponse,
  GetExtensionsResponse,
  GetSessionRequest,
  GetSessionResponse,
  GetToolsRequest,
  GetToolsResponse,
  ImportSessionRequest,
  ImportSessionResponse,
  ListSessionsResponse,
  ReadResourceRequest,
  ReadResourceResponse,
  RemoveExtensionRequest,
  UpdateWorkingDirRequest,
} from './types.gen.js';
import {
  zExportSessionResponse,
  zGetExtensionsResponse,
  zGetSessionResponse,
  zGetToolsResponse,
  zImportSessionResponse,
  zListSessionsResponse,
  zReadResourceResponse,
} from './zod.gen.js';

/**
 * Typed client for Goose custom extension methods.
 * Wraps an ExtMethodProvider (e.g. ClientSideConnection) with proper types and Zod validation.
 */
export class GooseClient {
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

  async toolCall(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/tool/call", {});
  }

  async providerUpdate(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/provider/update", {});
  }

  async containerSet(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/container/set", {});
  }

  async appsList(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/apps/list", {});
  }

  async appsExport(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/apps/export", {});
  }

  async appsImport(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/apps/import", {});
  }

  async configProviders(): Promise<Record<string, unknown>> {
    return await this.conn.extMethod("_goose/config/providers", {});
  }
}
