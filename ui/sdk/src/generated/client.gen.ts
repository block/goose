// This file is auto-generated — do not edit manually.

export interface ExtMethodProvider {
  extMethod(
    method: string,
    params: Record<string, unknown>,
  ): Promise<Record<string, unknown>>;
}

import type { Client } from "@agentclientprotocol/sdk";
import type {
  AddConfigExtensionRequest_unstable,
  AddSessionExtensionRequest_unstable,
  ArchiveSessionRequest_unstable,
  CanonicalModelInfoRequest_unstable,
  CanonicalModelInfoResponse_unstable,
  ConfigReadAllRequest_unstable,
  ConfigReadAllResponse_unstable,
  ConfigReadRequest_unstable,
  ConfigReadResponse_unstable,
  ConfigRemoveRequest_unstable,
  ConfigUpsertRequest_unstable,
  CreateSourceRequest_unstable,
  CreateSourceResponse_unstable,
  CustomProviderCreateRequest_unstable,
  CustomProviderCreateResponse_unstable,
  CustomProviderDeleteRequest_unstable,
  CustomProviderDeleteResponse_unstable,
  CustomProviderReadRequest_unstable,
  CustomProviderReadResponse_unstable,
  CustomProviderUpdateRequest_unstable,
  CustomProviderUpdateResponse_unstable,
  DefaultsClearRequest_unstable,
  DefaultsReadRequest_unstable,
  DefaultsReadResponse_unstable,
  DefaultsSaveRequest_unstable,
  DeleteSessionRequest,
  DeleteSourceRequest_unstable,
  DiagnosticsGetRequest_unstable,
  DiagnosticsGetResponse_unstable,
  DictationConfigRequest_unstable,
  DictationConfigResponse_unstable,
  DictationModelSelectRequest_unstable,
  DictationSecretDeleteRequest_unstable,
  DictationSecretSaveRequest_unstable,
  DictationTranscribeRequest_unstable,
  DictationTranscribeResponse_unstable,
  ExportSessionRequest_unstable,
  ExportSessionResponse_unstable,
  ExportSourceRequest_unstable,
  ExportSourceResponse_unstable,
  GetAvailableExtensionsRequest_unstable,
  GetAvailableExtensionsResponse_unstable,
  GetConfigExtensionsRequest_unstable,
  GetConfigExtensionsResponse_unstable,
  GetPromptRequest_unstable,
  GetPromptResponse_unstable,
  GetSessionExtensionsRequest_unstable,
  GetSessionExtensionsResponse_unstable,
  GetSessionInfoRequest_unstable,
  GetSessionInfoResponse_unstable,
  GetToolsRequest_unstable,
  GetToolsResponse_unstable,
  GoslingSessionNotification_unstable,
  GoslingToolCallRequest_unstable,
  GoslingToolCallResponse_unstable,
  ImportSessionRequest_unstable,
  ImportSessionResponse_unstable,
  ImportSourcesRequest_unstable,
  ImportSourcesResponse_unstable,
  ListAgentMentionsRequest_unstable,
  ListAgentMentionsResponse_unstable,
  ListPromptsRequest_unstable,
  ListPromptsResponse_unstable,
  ListProvidersRequest_unstable,
  ListProvidersResponse_unstable,
  ListSlashCommandsRequest_unstable,
  ListSlashCommandsResponse_unstable,
  ListSourcesRequest_unstable,
  ListSourcesResponse_unstable,
  OnboardingImportApplyRequest_unstable,
  OnboardingImportApplyResponse_unstable,
  OnboardingImportScanRequest_unstable,
  OnboardingImportScanResponse_unstable,
  PreferencesReadRequest_unstable,
  PreferencesReadResponse_unstable,
  PreferencesRemoveRequest_unstable,
  PreferencesSaveRequest_unstable,
  PromptOperationResponse_unstable,
  ProviderCatalogListRequest_unstable,
  ProviderCatalogListResponse_unstable,
  ProviderCatalogTemplateRequest_unstable,
  ProviderCatalogTemplateResponse_unstable,
  ProviderConfigAuthenticateRequest_unstable,
  ProviderConfigChangeResponse_unstable,
  ProviderConfigDeleteRequest_unstable,
  ProviderConfigReadRequest_unstable,
  ProviderConfigReadResponse_unstable,
  ProviderConfigSaveRequest_unstable,
  ProviderConfigStatusRequest_unstable,
  ProviderConfigStatusResponse_unstable,
  ProviderSecretDeleteRequest_unstable,
  ProviderSecretsListRequest_unstable,
  ProviderSecretsListResponse_unstable,
  ProviderSetupCatalogListRequest_unstable,
  ProviderSetupCatalogListResponse_unstable,
  ProviderSupportedModelsListRequest_unstable,
  ProviderSupportedModelsListResponse_unstable,
  ReadResourceRequest_unstable,
  ReadResourceResponse_unstable,
  RefreshProviderInventoryRequest_unstable,
  RefreshProviderInventoryResponse_unstable,
  RemoveConfigExtensionRequest_unstable,
  RemoveSessionExtensionRequest_unstable,
  RenameSessionRequest_unstable,
  ResetPromptRequest_unstable,
  SavePromptRequest_unstable,
  SetConfigExtensionEnabledRequest_unstable,
  SetSessionSystemPromptRequest_unstable,
  SetToolPermissionsRequest_unstable,
  SetToolPermissionsResponse_unstable,
  ShareSessionNostrRequest_unstable,
  ShareSessionNostrResponse_unstable,
  SteerSessionRequest_unstable,
  SteerSessionResponse_unstable,
  TruncateSessionConversationRequest_unstable,
  UnarchiveSessionRequest_unstable,
  UpdateSessionProjectRequest_unstable,
  UpdateSourceRequest_unstable,
  UpdateSourceResponse_unstable,
  UpdateWorkingDirRequest_unstable,
} from './types.gen.js';
import {
  zCanonicalModelInfoResponse_unstable,
  zConfigReadAllResponse_unstable,
  zConfigReadResponse_unstable,
  zCreateSourceResponse_unstable,
  zCustomProviderCreateResponse_unstable,
  zCustomProviderDeleteResponse_unstable,
  zCustomProviderReadResponse_unstable,
  zCustomProviderUpdateResponse_unstable,
  zDefaultsReadResponse_unstable,
  zDiagnosticsGetResponse_unstable,
  zDictationConfigResponse_unstable,
  zDictationTranscribeResponse_unstable,
  zExportSessionResponse_unstable,
  zExportSourceResponse_unstable,
  zGetAvailableExtensionsResponse_unstable,
  zGetConfigExtensionsResponse_unstable,
  zGetPromptResponse_unstable,
  zGetSessionExtensionsResponse_unstable,
  zGetSessionInfoResponse_unstable,
  zGetToolsResponse_unstable,
  zGoslingSessionNotification_unstable,
  zGoslingToolCallResponse_unstable,
  zImportSessionResponse_unstable,
  zImportSourcesResponse_unstable,
  zListAgentMentionsResponse_unstable,
  zListPromptsResponse_unstable,
  zListProvidersResponse_unstable,
  zListSlashCommandsResponse_unstable,
  zListSourcesResponse_unstable,
  zOnboardingImportApplyResponse_unstable,
  zOnboardingImportScanResponse_unstable,
  zPreferencesReadResponse_unstable,
  zPromptOperationResponse_unstable,
  zProviderCatalogListResponse_unstable,
  zProviderCatalogTemplateResponse_unstable,
  zProviderConfigChangeResponse_unstable,
  zProviderConfigReadResponse_unstable,
  zProviderConfigStatusResponse_unstable,
  zProviderSecretsListResponse_unstable,
  zProviderSetupCatalogListResponse_unstable,
  zProviderSupportedModelsListResponse_unstable,
  zReadResourceResponse_unstable,
  zRefreshProviderInventoryResponse_unstable,
  zSetToolPermissionsResponse_unstable,
  zShareSessionNostrResponse_unstable,
  zSteerSessionResponse_unstable,
  zUpdateSourceResponse_unstable,
} from './zod.gen.js';

export class GoslingExtClient {
  constructor(private conn: ExtMethodProvider) {}

  async sessionExtensionsAdd_unstable(
    params: AddSessionExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/extensions/add",
      params,
    );
  }

  async sessionExtensionsRemove_unstable(
    params: RemoveSessionExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/extensions/remove",
      params,
    );
  }

  async toolsList_unstable(
    params: GetToolsRequest_unstable,
  ): Promise<GetToolsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/tools/list",
      params,
    );
    return zGetToolsResponse_unstable.parse(raw) as GetToolsResponse_unstable;
  }

  async toolsPermissionsSet_unstable(
    params: SetToolPermissionsRequest_unstable,
  ): Promise<SetToolPermissionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/tools/permissions/set",
      params,
    );
    return zSetToolPermissionsResponse_unstable.parse(
      raw,
    ) as SetToolPermissionsResponse_unstable;
  }

  async toolsCall_unstable(
    params: GoslingToolCallRequest_unstable,
  ): Promise<GoslingToolCallResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/tools/call",
      params,
    );
    return zGoslingToolCallResponse_unstable.parse(
      raw,
    ) as GoslingToolCallResponse_unstable;
  }

  async resourcesRead_unstable(
    params: ReadResourceRequest_unstable,
  ): Promise<ReadResourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/resources/read",
      params,
    );
    return zReadResourceResponse_unstable.parse(
      raw,
    ) as ReadResourceResponse_unstable;
  }

  async sessionWorkingDirUpdate_unstable(
    params: UpdateWorkingDirRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/working-dir/update",
      params,
    );
  }

  async sessionSystemPromptSet_unstable(
    params: SetSessionSystemPromptRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/system-prompt/set",
      params,
    );
  }

  async sessionSteer_unstable(
    params: SteerSessionRequest_unstable,
  ): Promise<SteerSessionResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/steer",
      params,
    );
    return zSteerSessionResponse_unstable.parse(
      raw,
    ) as SteerSessionResponse_unstable;
  }

  async diagnosticsGet_unstable(
    params: DiagnosticsGetRequest_unstable,
  ): Promise<DiagnosticsGetResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/diagnostics/get",
      params,
    );
    return zDiagnosticsGetResponse_unstable.parse(
      raw,
    ) as DiagnosticsGetResponse_unstable;
  }

  async configPromptsList_unstable(
    params: ListPromptsRequest_unstable,
  ): Promise<ListPromptsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/prompts/list",
      params,
    );
    return zListPromptsResponse_unstable.parse(
      raw,
    ) as ListPromptsResponse_unstable;
  }

  async configPromptsGet_unstable(
    params: GetPromptRequest_unstable,
  ): Promise<GetPromptResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/prompts/get",
      params,
    );
    return zGetPromptResponse_unstable.parse(raw) as GetPromptResponse_unstable;
  }

  async configPromptsSave_unstable(
    params: SavePromptRequest_unstable,
  ): Promise<PromptOperationResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/prompts/save",
      params,
    );
    return zPromptOperationResponse_unstable.parse(
      raw,
    ) as PromptOperationResponse_unstable;
  }

  async configPromptsReset_unstable(
    params: ResetPromptRequest_unstable,
  ): Promise<PromptOperationResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/prompts/reset",
      params,
    );
    return zPromptOperationResponse_unstable.parse(
      raw,
    ) as PromptOperationResponse_unstable;
  }

  async sessionDelete(params: DeleteSessionRequest): Promise<void> {
    await this.conn.extMethod("session/delete", params);
  }

  async configExtensionsList_unstable(
    params: GetConfigExtensionsRequest_unstable,
  ): Promise<GetConfigExtensionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/extensions/list",
      params,
    );
    return zGetConfigExtensionsResponse_unstable.parse(
      raw,
    ) as GetConfigExtensionsResponse_unstable;
  }

  async extensionsAvailable_unstable(
    params: GetAvailableExtensionsRequest_unstable,
  ): Promise<GetAvailableExtensionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/extensions/available",
      params,
    );
    return zGetAvailableExtensionsResponse_unstable.parse(
      raw,
    ) as GetAvailableExtensionsResponse_unstable;
  }

  async configExtensionsAdd_unstable(
    params: AddConfigExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/config/extensions/add",
      params,
    );
  }

  async configExtensionsRemove_unstable(
    params: RemoveConfigExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/config/extensions/remove",
      params,
    );
  }

  async configExtensionsSetEnabled_unstable(
    params: SetConfigExtensionEnabledRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/config/extensions/set-enabled",
      params,
    );
  }

  async sessionExtensionsList_unstable(
    params: GetSessionExtensionsRequest_unstable,
  ): Promise<GetSessionExtensionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/extensions/list",
      params,
    );
    return zGetSessionExtensionsResponse_unstable.parse(
      raw,
    ) as GetSessionExtensionsResponse_unstable;
  }

  async providersList_unstable(
    params: ListProvidersRequest_unstable,
  ): Promise<ListProvidersResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/list",
      params,
    );
    return zListProvidersResponse_unstable.parse(
      raw,
    ) as ListProvidersResponse_unstable;
  }

  async providersSupportedModelsList_unstable(
    params: ProviderSupportedModelsListRequest_unstable,
  ): Promise<ProviderSupportedModelsListResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/supported-models/list",
      params,
    );
    return zProviderSupportedModelsListResponse_unstable.parse(
      raw,
    ) as ProviderSupportedModelsListResponse_unstable;
  }

  async providersCatalogList_unstable(
    params: ProviderCatalogListRequest_unstable,
  ): Promise<ProviderCatalogListResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/catalog/list",
      params,
    );
    return zProviderCatalogListResponse_unstable.parse(
      raw,
    ) as ProviderCatalogListResponse_unstable;
  }

  async providersSetupCatalogList_unstable(
    params: ProviderSetupCatalogListRequest_unstable,
  ): Promise<ProviderSetupCatalogListResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/setup/catalog/list",
      params,
    );
    return zProviderSetupCatalogListResponse_unstable.parse(
      raw,
    ) as ProviderSetupCatalogListResponse_unstable;
  }

  async providersCatalogTemplate_unstable(
    params: ProviderCatalogTemplateRequest_unstable,
  ): Promise<ProviderCatalogTemplateResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/catalog/template",
      params,
    );
    return zProviderCatalogTemplateResponse_unstable.parse(
      raw,
    ) as ProviderCatalogTemplateResponse_unstable;
  }

  async providersCustomCreate_unstable(
    params: CustomProviderCreateRequest_unstable,
  ): Promise<CustomProviderCreateResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/custom/create",
      params,
    );
    return zCustomProviderCreateResponse_unstable.parse(
      raw,
    ) as CustomProviderCreateResponse_unstable;
  }

  async providersCustomRead_unstable(
    params: CustomProviderReadRequest_unstable,
  ): Promise<CustomProviderReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/custom/read",
      params,
    );
    return zCustomProviderReadResponse_unstable.parse(
      raw,
    ) as CustomProviderReadResponse_unstable;
  }

  async providersCustomUpdate_unstable(
    params: CustomProviderUpdateRequest_unstable,
  ): Promise<CustomProviderUpdateResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/custom/update",
      params,
    );
    return zCustomProviderUpdateResponse_unstable.parse(
      raw,
    ) as CustomProviderUpdateResponse_unstable;
  }

  async providersCustomDelete_unstable(
    params: CustomProviderDeleteRequest_unstable,
  ): Promise<CustomProviderDeleteResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/custom/delete",
      params,
    );
    return zCustomProviderDeleteResponse_unstable.parse(
      raw,
    ) as CustomProviderDeleteResponse_unstable;
  }

  async providersInventoryRefresh_unstable(
    params: RefreshProviderInventoryRequest_unstable,
  ): Promise<RefreshProviderInventoryResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/inventory/refresh",
      params,
    );
    return zRefreshProviderInventoryResponse_unstable.parse(
      raw,
    ) as RefreshProviderInventoryResponse_unstable;
  }

  async providersConfigRead_unstable(
    params: ProviderConfigReadRequest_unstable,
  ): Promise<ProviderConfigReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/config/read",
      params,
    );
    return zProviderConfigReadResponse_unstable.parse(
      raw,
    ) as ProviderConfigReadResponse_unstable;
  }

  async providersConfigStatus_unstable(
    params: ProviderConfigStatusRequest_unstable,
  ): Promise<ProviderConfigStatusResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/config/status",
      params,
    );
    return zProviderConfigStatusResponse_unstable.parse(
      raw,
    ) as ProviderConfigStatusResponse_unstable;
  }

  async providersConfigSave_unstable(
    params: ProviderConfigSaveRequest_unstable,
  ): Promise<ProviderConfigChangeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/config/save",
      params,
    );
    return zProviderConfigChangeResponse_unstable.parse(
      raw,
    ) as ProviderConfigChangeResponse_unstable;
  }

  async providersConfigDelete_unstable(
    params: ProviderConfigDeleteRequest_unstable,
  ): Promise<ProviderConfigChangeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/config/delete",
      params,
    );
    return zProviderConfigChangeResponse_unstable.parse(
      raw,
    ) as ProviderConfigChangeResponse_unstable;
  }

  async providersConfigAuthenticate_unstable(
    params: ProviderConfigAuthenticateRequest_unstable,
  ): Promise<ProviderConfigChangeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/config/authenticate",
      params,
    );
    return zProviderConfigChangeResponse_unstable.parse(
      raw,
    ) as ProviderConfigChangeResponse_unstable;
  }

  async providersSecretsList_unstable(
    params: ProviderSecretsListRequest_unstable,
  ): Promise<ProviderSecretsListResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/secrets/list",
      params,
    );
    return zProviderSecretsListResponse_unstable.parse(
      raw,
    ) as ProviderSecretsListResponse_unstable;
  }

  async providersSecretsDelete_unstable(
    params: ProviderSecretDeleteRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/providers/secrets/delete",
      params,
    );
  }

  async providersCanonicalModelInfo_unstable(
    params: CanonicalModelInfoRequest_unstable,
  ): Promise<CanonicalModelInfoResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/providers/canonical-model-info",
      params,
    );
    return zCanonicalModelInfoResponse_unstable.parse(
      raw,
    ) as CanonicalModelInfoResponse_unstable;
  }

  async preferencesRead_unstable(
    params: PreferencesReadRequest_unstable,
  ): Promise<PreferencesReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/preferences/read",
      params,
    );
    return zPreferencesReadResponse_unstable.parse(
      raw,
    ) as PreferencesReadResponse_unstable;
  }

  async preferencesSave_unstable(
    params: PreferencesSaveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/preferences/save", params);
  }

  async preferencesRemove_unstable(
    params: PreferencesRemoveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/preferences/remove", params);
  }

  async configRead_unstable(
    params: ConfigReadRequest_unstable,
  ): Promise<ConfigReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/read",
      params,
    );
    return zConfigReadResponse_unstable.parse(
      raw,
    ) as ConfigReadResponse_unstable;
  }

  async configUpsert_unstable(
    params: ConfigUpsertRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/config/upsert", params);
  }

  async configRemove_unstable(
    params: ConfigRemoveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/config/remove", params);
  }

  async configReadAll_unstable(
    params: ConfigReadAllRequest_unstable,
  ): Promise<ConfigReadAllResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/config/read-all",
      params,
    );
    return zConfigReadAllResponse_unstable.parse(
      raw,
    ) as ConfigReadAllResponse_unstable;
  }

  async defaultsRead_unstable(
    params: DefaultsReadRequest_unstable,
  ): Promise<DefaultsReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/defaults/read",
      params,
    );
    return zDefaultsReadResponse_unstable.parse(
      raw,
    ) as DefaultsReadResponse_unstable;
  }

  async defaultsSave_unstable(
    params: DefaultsSaveRequest_unstable,
  ): Promise<DefaultsReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/defaults/save",
      params,
    );
    return zDefaultsReadResponse_unstable.parse(
      raw,
    ) as DefaultsReadResponse_unstable;
  }

  async defaultsClear_unstable(
    params: DefaultsClearRequest_unstable,
  ): Promise<DefaultsReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/defaults/clear",
      params,
    );
    return zDefaultsReadResponse_unstable.parse(
      raw,
    ) as DefaultsReadResponse_unstable;
  }

  async onboardingImportScan_unstable(
    params: OnboardingImportScanRequest_unstable,
  ): Promise<OnboardingImportScanResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/onboarding/import/scan",
      params,
    );
    return zOnboardingImportScanResponse_unstable.parse(
      raw,
    ) as OnboardingImportScanResponse_unstable;
  }

  async onboardingImportApply_unstable(
    params: OnboardingImportApplyRequest_unstable,
  ): Promise<OnboardingImportApplyResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/onboarding/import/apply",
      params,
    );
    return zOnboardingImportApplyResponse_unstable.parse(
      raw,
    ) as OnboardingImportApplyResponse_unstable;
  }

  async sessionExport_unstable(
    params: ExportSessionRequest_unstable,
  ): Promise<ExportSessionResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/export",
      params,
    );
    return zExportSessionResponse_unstable.parse(
      raw,
    ) as ExportSessionResponse_unstable;
  }

  async sessionImport_unstable(
    params: ImportSessionRequest_unstable,
  ): Promise<ImportSessionResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/import",
      params,
    );
    return zImportSessionResponse_unstable.parse(
      raw,
    ) as ImportSessionResponse_unstable;
  }

  async sessionShareNostr_unstable(
    params: ShareSessionNostrRequest_unstable,
  ): Promise<ShareSessionNostrResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/share/nostr",
      params,
    );
    return zShareSessionNostrResponse_unstable.parse(
      raw,
    ) as ShareSessionNostrResponse_unstable;
  }

  async sessionInfo_unstable(
    params: GetSessionInfoRequest_unstable,
  ): Promise<GetSessionInfoResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/session/info",
      params,
    );
    return zGetSessionInfoResponse_unstable.parse(
      raw,
    ) as GetSessionInfoResponse_unstable;
  }

  async sessionConversationTruncate_unstable(
    params: TruncateSessionConversationRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/conversation/truncate",
      params,
    );
  }

  async sessionProjectUpdate_unstable(
    params: UpdateSessionProjectRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/session/project/update",
      params,
    );
  }

  async sessionRename_unstable(
    params: RenameSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/session/rename", params);
  }

  async sessionArchive_unstable(
    params: ArchiveSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/session/archive", params);
  }

  async sessionUnarchive_unstable(
    params: UnarchiveSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/session/unarchive", params);
  }

  async sourcesCreate_unstable(
    params: CreateSourceRequest_unstable,
  ): Promise<CreateSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/sources/create",
      params,
    );
    return zCreateSourceResponse_unstable.parse(
      raw,
    ) as CreateSourceResponse_unstable;
  }

  async sourcesList_unstable(
    params: ListSourcesRequest_unstable,
  ): Promise<ListSourcesResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/sources/list",
      params,
    );
    return zListSourcesResponse_unstable.parse(
      raw,
    ) as ListSourcesResponse_unstable;
  }

  async agentMentionsList_unstable(
    params: ListAgentMentionsRequest_unstable,
  ): Promise<ListAgentMentionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/agent-mentions/list",
      params,
    );
    return zListAgentMentionsResponse_unstable.parse(
      raw,
    ) as ListAgentMentionsResponse_unstable;
  }

  async slashCommandsList_unstable(
    params: ListSlashCommandsRequest_unstable,
  ): Promise<ListSlashCommandsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/slash-commands/list",
      params,
    );
    return zListSlashCommandsResponse_unstable.parse(
      raw,
    ) as ListSlashCommandsResponse_unstable;
  }

  async sourcesUpdate_unstable(
    params: UpdateSourceRequest_unstable,
  ): Promise<UpdateSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/sources/update",
      params,
    );
    return zUpdateSourceResponse_unstable.parse(
      raw,
    ) as UpdateSourceResponse_unstable;
  }

  async sourcesDelete_unstable(
    params: DeleteSourceRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_gosling/unstable/sources/delete", params);
  }

  async sourcesExport_unstable(
    params: ExportSourceRequest_unstable,
  ): Promise<ExportSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/sources/export",
      params,
    );
    return zExportSourceResponse_unstable.parse(
      raw,
    ) as ExportSourceResponse_unstable;
  }

  async sourcesImport_unstable(
    params: ImportSourcesRequest_unstable,
  ): Promise<ImportSourcesResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/sources/import",
      params,
    );
    return zImportSourcesResponse_unstable.parse(
      raw,
    ) as ImportSourcesResponse_unstable;
  }

  async dictationTranscribe_unstable(
    params: DictationTranscribeRequest_unstable,
  ): Promise<DictationTranscribeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/dictation/transcribe",
      params,
    );
    return zDictationTranscribeResponse_unstable.parse(
      raw,
    ) as DictationTranscribeResponse_unstable;
  }

  async dictationConfig_unstable(
    params: DictationConfigRequest_unstable,
  ): Promise<DictationConfigResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_gosling/unstable/dictation/config",
      params,
    );
    return zDictationConfigResponse_unstable.parse(
      raw,
    ) as DictationConfigResponse_unstable;
  }

  async dictationSecretSave_unstable(
    params: DictationSecretSaveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/dictation/secret/save",
      params,
    );
  }

  async dictationSecretDelete_unstable(
    params: DictationSecretDeleteRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/dictation/secret/delete",
      params,
    );
  }

  async dictationModelsSelect_unstable(
    params: DictationModelSelectRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_gosling/unstable/dictation/models/select",
      params,
    );
  }
}

export interface GoslingExtNotifications {
  unstable_sessionUpdate?: (
    notification: GoslingSessionNotification_unstable,
  ) => Promise<void>;
}

export interface GoslingExtAgentRequests {}

export type GoslingClientCallbacks = Omit<
  Client,
  "extNotification" | "extMethod"
> &
  Partial<Pick<Client, "extNotification" | "extMethod">> &
  GoslingExtNotifications &
  GoslingExtAgentRequests;

export function installGoslingExtNotificationDispatcher(
  callbacks: GoslingClientCallbacks,
): Client {
  const dispatcher: Pick<Client, "extNotification"> = {
    extNotification: async (method, params) => {
      switch (method) {
        case "_gosling/unstable/session/update": {
          const parsed = zGoslingSessionNotification_unstable.parse(
            params,
          ) as GoslingSessionNotification_unstable;
          await callbacks.unstable_sessionUpdate?.(parsed);
          return;
        }
        default:
          await callbacks.extNotification?.(method, params);
          return;
      }
    },
  };
  return new Proxy(callbacks, {
    get(target, property) {
      if (property === "extNotification") {
        return dispatcher.extNotification;
      }

      const value = Reflect.get(target, property, target);
      return typeof value === "function" ? value.bind(target) : value;
    },
  }) as Client;
}

export function installGoslingExtAgentRequestDispatcher(
  callbacks: GoslingClientCallbacks,
): Client {
  const dispatcher: Pick<Client, "extMethod"> = {
    extMethod: async (method, params) => {
      switch (method) {
        default:
          if (callbacks.extMethod) {
            return await callbacks.extMethod(method, params);
          }
          throw new Error(`unhandled ext method: ${method}`);
      }
    },
  };
  return new Proxy(callbacks, {
    get(target, property) {
      if (property === "extMethod") {
        return dispatcher.extMethod;
      }

      const value = Reflect.get(target, property, target);
      return typeof value === "function" ? value.bind(target) : value;
    },
  }) as Client;
}
