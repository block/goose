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
  zReadResourceResponse,
} from './zod.gen.js';

export class GooseExtClient {
  constructor(private conn: ExtMethodProvider) {}

  async GooseExtensionsAdd(params: AddExtensionRequest): Promise<void> {
    await this.conn.extMethod("_goose/extensions/add", params);
  }

  async GooseExtensionsRemove(params: RemoveExtensionRequest): Promise<void> {
    await this.conn.extMethod("_goose/extensions/remove", params);
  }

  async GooseTools(params: GetToolsRequest): Promise<GetToolsResponse> {
    const raw = await this.conn.extMethod("_goose/tools", params);
    return zGetToolsResponse.parse(raw) as GetToolsResponse;
  }

  async GooseResourceRead(
    params: ReadResourceRequest,
  ): Promise<ReadResourceResponse> {
    const raw = await this.conn.extMethod("_goose/resource/read", params);
    return zReadResourceResponse.parse(raw) as ReadResourceResponse;
  }

  async GooseWorkingDirUpdate(params: UpdateWorkingDirRequest): Promise<void> {
    await this.conn.extMethod("_goose/working_dir/update", params);
  }

  async sessionGet(params: GetSessionRequest): Promise<GetSessionResponse> {
    const raw = await this.conn.extMethod("session/get", params);
    return zGetSessionResponse.parse(raw) as GetSessionResponse;
  }

  async sessionDelete(params: DeleteSessionRequest): Promise<void> {
    await this.conn.extMethod("session/delete", params);
  }

  async GooseSessionExport(
    params: ExportSessionRequest,
  ): Promise<ExportSessionResponse> {
    const raw = await this.conn.extMethod("_goose/session/export", params);
    return zExportSessionResponse.parse(raw) as ExportSessionResponse;
  }

  async GooseSessionImport(
    params: ImportSessionRequest,
  ): Promise<ImportSessionResponse> {
    const raw = await this.conn.extMethod("_goose/session/import", params);
    return zImportSessionResponse.parse(raw) as ImportSessionResponse;
  }

  async GooseConfigExtensions(): Promise<GetExtensionsResponse> {
    const raw = await this.conn.extMethod("_goose/config/extensions", {});
    return zGetExtensionsResponse.parse(raw) as GetExtensionsResponse;
  }
}
