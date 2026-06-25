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
  DecodeRecipeRequest_unstable,
  DecodeRecipeResponse_unstable,
  DefaultsReadRequest_unstable,
  DefaultsReadResponse_unstable,
  DefaultsSaveRequest_unstable,
  DeleteRecipeRequest_unstable,
  DeleteSessionRequest,
  DeleteSourceRequest_unstable,
  DiagnosticsGetRequest_unstable,
  DiagnosticsGetResponse_unstable,
  DictationConfigRequest_unstable,
  DictationConfigResponse_unstable,
  DictationModelCancelRequest_unstable,
  DictationModelDeleteRequest_unstable,
  DictationModelDownloadProgressRequest_unstable,
  DictationModelDownloadProgressResponse_unstable,
  DictationModelDownloadRequest_unstable,
  DictationModelSelectRequest_unstable,
  DictationModelsListRequest_unstable,
  DictationModelsListResponse_unstable,
  DictationSecretDeleteRequest_unstable,
  DictationSecretSaveRequest_unstable,
  DictationTranscribeRequest_unstable,
  DictationTranscribeResponse_unstable,
  EncodeRecipeRequest_unstable,
  EncodeRecipeResponse_unstable,
  ExportSessionRequest_unstable,
  ExportSessionResponse_unstable,
  ExportSourceRequest_unstable,
  ExportSourceResponse_unstable,
  GetAvailableExtensionsRequest_unstable,
  GetAvailableExtensionsResponse_unstable,
  GetConfigExtensionsRequest_unstable,
  GetConfigExtensionsResponse_unstable,
  GetSessionExtensionsRequest_unstable,
  GetSessionExtensionsResponse_unstable,
  GetSessionInfoRequest_unstable,
  GetSessionInfoResponse_unstable,
  GetToolsRequest_unstable,
  GetToolsResponse_unstable,
  GooseSessionNotification_unstable,
  GooseToolCallRequest_unstable,
  GooseToolCallResponse_unstable,
  ImportSessionRequest_unstable,
  ImportSessionResponse_unstable,
  ImportSourcesRequest_unstable,
  ImportSourcesResponse_unstable,
  ListProvidersRequest_unstable,
  ListProvidersResponse_unstable,
  ListRecipesRequest_unstable,
  ListRecipesResponse_unstable,
  ListSourcesRequest_unstable,
  ListSourcesResponse_unstable,
  OnboardingImportApplyRequest_unstable,
  OnboardingImportApplyResponse_unstable,
  OnboardingImportScanRequest_unstable,
  OnboardingImportScanResponse_unstable,
  ParseRecipeRequest_unstable,
  ParseRecipeResponse_unstable,
  PreferencesReadRequest_unstable,
  PreferencesReadResponse_unstable,
  PreferencesRemoveRequest_unstable,
  PreferencesSaveRequest_unstable,
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
  ProviderSetupCatalogListRequest_unstable,
  ProviderSetupCatalogListResponse_unstable,
  ProviderSupportedModelsListRequest_unstable,
  ProviderSupportedModelsListResponse_unstable,
  ReadResourceRequest_unstable,
  ReadResourceResponse_unstable,
  RecipeParamsResponse_unstable,
  RecipeToYamlRequest_unstable,
  RecipeToYamlResponse_unstable,
  RefreshProviderInventoryRequest_unstable,
  RefreshProviderInventoryResponse_unstable,
  RemoveConfigExtensionRequest_unstable,
  RemoveSessionExtensionRequest_unstable,
  RenameSessionRequest_unstable,
  RequestRecipeParams_unstable,
  SaveRecipeRequest_unstable,
  SaveRecipeResponse_unstable,
  ScanRecipeRequest_unstable,
  ScanRecipeResponse_unstable,
  ScheduleRecipeRequest_unstable,
  SetConfigExtensionEnabledRequest_unstable,
  SetRecipeSlashCommandRequest_unstable,
  SetSessionSystemPromptRequest_unstable,
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
  zCreateSourceResponse_unstable,
  zCustomProviderCreateResponse_unstable,
  zCustomProviderDeleteResponse_unstable,
  zCustomProviderReadResponse_unstable,
  zCustomProviderUpdateResponse_unstable,
  zDecodeRecipeResponse_unstable,
  zDefaultsReadResponse_unstable,
  zDiagnosticsGetResponse_unstable,
  zDictationConfigResponse_unstable,
  zDictationModelDownloadProgressResponse_unstable,
  zDictationModelsListResponse_unstable,
  zDictationTranscribeResponse_unstable,
  zEncodeRecipeResponse_unstable,
  zExportSessionResponse_unstable,
  zExportSourceResponse_unstable,
  zGetAvailableExtensionsResponse_unstable,
  zGetConfigExtensionsResponse_unstable,
  zGetSessionExtensionsResponse_unstable,
  zGetSessionInfoResponse_unstable,
  zGetToolsResponse_unstable,
  zGooseSessionNotification_unstable,
  zGooseToolCallResponse_unstable,
  zImportSessionResponse_unstable,
  zImportSourcesResponse_unstable,
  zListProvidersResponse_unstable,
  zListRecipesResponse_unstable,
  zListSourcesResponse_unstable,
  zOnboardingImportApplyResponse_unstable,
  zOnboardingImportScanResponse_unstable,
  zParseRecipeResponse_unstable,
  zPreferencesReadResponse_unstable,
  zProviderCatalogListResponse_unstable,
  zProviderCatalogTemplateResponse_unstable,
  zProviderConfigChangeResponse_unstable,
  zProviderConfigReadResponse_unstable,
  zProviderConfigStatusResponse_unstable,
  zProviderSetupCatalogListResponse_unstable,
  zProviderSupportedModelsListResponse_unstable,
  zReadResourceResponse_unstable,
  zRecipeToYamlResponse_unstable,
  zRefreshProviderInventoryResponse_unstable,
  zRequestRecipeParams_unstable,
  zSaveRecipeResponse_unstable,
  zScanRecipeResponse_unstable,
  zSteerSessionResponse_unstable,
  zUpdateSourceResponse_unstable,
} from './zod.gen.js';

export class GooseExtClient {
  constructor(private conn: ExtMethodProvider) {}

  async sessionExtensionsAdd_unstable(
    params: AddSessionExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/session/extensions/add", params);
  }

  async sessionExtensionsRemove_unstable(
    params: RemoveSessionExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/session/extensions/remove",
      params,
    );
  }

  async toolsList_unstable(
    params: GetToolsRequest_unstable,
  ): Promise<GetToolsResponse_unstable> {
    const raw = await this.conn.extMethod("_goose/unstable/tools/list", params);
    return zGetToolsResponse_unstable.parse(raw) as GetToolsResponse_unstable;
  }

  async toolsCall_unstable(
    params: GooseToolCallRequest_unstable,
  ): Promise<GooseToolCallResponse_unstable> {
    const raw = await this.conn.extMethod("_goose/unstable/tools/call", params);
    return zGooseToolCallResponse_unstable.parse(
      raw,
    ) as GooseToolCallResponse_unstable;
  }

  async resourcesRead_unstable(
    params: ReadResourceRequest_unstable,
  ): Promise<ReadResourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/resources/read",
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
      "_goose/unstable/session/working-dir/update",
      params,
    );
  }

  async sessionSystemPromptSet_unstable(
    params: SetSessionSystemPromptRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/session/system-prompt/set",
      params,
    );
  }

  async sessionSteer_unstable(
    params: SteerSessionRequest_unstable,
  ): Promise<SteerSessionResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/session/steer",
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
      "_goose/unstable/diagnostics/get",
      params,
    );
    return zDiagnosticsGetResponse_unstable.parse(
      raw,
    ) as DiagnosticsGetResponse_unstable;
  }

  async sessionDelete(params: DeleteSessionRequest): Promise<void> {
    await this.conn.extMethod("session/delete", params);
  }

  async configExtensionsList_unstable(
    params: GetConfigExtensionsRequest_unstable,
  ): Promise<GetConfigExtensionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/config/extensions/list",
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
      "_goose/unstable/extensions/available",
      params,
    );
    return zGetAvailableExtensionsResponse_unstable.parse(
      raw,
    ) as GetAvailableExtensionsResponse_unstable;
  }

  async configExtensionsAdd_unstable(
    params: AddConfigExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/config/extensions/add", params);
  }

  async configExtensionsRemove_unstable(
    params: RemoveConfigExtensionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/config/extensions/remove",
      params,
    );
  }

  async configExtensionsSetEnabled_unstable(
    params: SetConfigExtensionEnabledRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/config/extensions/set-enabled",
      params,
    );
  }

  async sessionExtensionsList_unstable(
    params: GetSessionExtensionsRequest_unstable,
  ): Promise<GetSessionExtensionsResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/session/extensions/list",
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
      "_goose/unstable/providers/list",
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
      "_goose/unstable/providers/supported-models/list",
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
      "_goose/unstable/providers/catalog/list",
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
      "_goose/unstable/providers/setup/catalog/list",
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
      "_goose/unstable/providers/catalog/template",
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
      "_goose/unstable/providers/custom/create",
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
      "_goose/unstable/providers/custom/read",
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
      "_goose/unstable/providers/custom/update",
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
      "_goose/unstable/providers/custom/delete",
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
      "_goose/unstable/providers/inventory/refresh",
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
      "_goose/unstable/providers/config/read",
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
      "_goose/unstable/providers/config/status",
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
      "_goose/unstable/providers/config/save",
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
      "_goose/unstable/providers/config/delete",
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
      "_goose/unstable/providers/config/authenticate",
      params,
    );
    return zProviderConfigChangeResponse_unstable.parse(
      raw,
    ) as ProviderConfigChangeResponse_unstable;
  }

  async preferencesRead_unstable(
    params: PreferencesReadRequest_unstable,
  ): Promise<PreferencesReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/preferences/read",
      params,
    );
    return zPreferencesReadResponse_unstable.parse(
      raw,
    ) as PreferencesReadResponse_unstable;
  }

  async preferencesSave_unstable(
    params: PreferencesSaveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/preferences/save", params);
  }

  async preferencesRemove_unstable(
    params: PreferencesRemoveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/preferences/remove", params);
  }

  async defaultsRead_unstable(
    params: DefaultsReadRequest_unstable,
  ): Promise<DefaultsReadResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/defaults/read",
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
      "_goose/unstable/defaults/save",
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
      "_goose/unstable/onboarding/import/scan",
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
      "_goose/unstable/onboarding/import/apply",
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
      "_goose/unstable/session/export",
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
      "_goose/unstable/session/import",
      params,
    );
    return zImportSessionResponse_unstable.parse(
      raw,
    ) as ImportSessionResponse_unstable;
  }

  async recipesEncode_unstable(
    params: EncodeRecipeRequest_unstable,
  ): Promise<EncodeRecipeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/encode",
      params,
    );
    return zEncodeRecipeResponse_unstable.parse(
      raw,
    ) as EncodeRecipeResponse_unstable;
  }

  async recipesDecode_unstable(
    params: DecodeRecipeRequest_unstable,
  ): Promise<DecodeRecipeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/decode",
      params,
    );
    return zDecodeRecipeResponse_unstable.parse(
      raw,
    ) as DecodeRecipeResponse_unstable;
  }

  async recipesScan_unstable(
    params: ScanRecipeRequest_unstable,
  ): Promise<ScanRecipeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/scan",
      params,
    );
    return zScanRecipeResponse_unstable.parse(
      raw,
    ) as ScanRecipeResponse_unstable;
  }

  async recipesList_unstable(
    params: ListRecipesRequest_unstable,
  ): Promise<ListRecipesResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/list",
      params,
    );
    return zListRecipesResponse_unstable.parse(
      raw,
    ) as ListRecipesResponse_unstable;
  }

  async recipesDelete_unstable(
    params: DeleteRecipeRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/recipes/delete", params);
  }

  async recipesSchedule_unstable(
    params: ScheduleRecipeRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/recipes/schedule", params);
  }

  async recipesSlashCommand_unstable(
    params: SetRecipeSlashCommandRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/recipes/slash-command", params);
  }

  async recipesSave_unstable(
    params: SaveRecipeRequest_unstable,
  ): Promise<SaveRecipeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/save",
      params,
    );
    return zSaveRecipeResponse_unstable.parse(
      raw,
    ) as SaveRecipeResponse_unstable;
  }

  async recipesParse_unstable(
    params: ParseRecipeRequest_unstable,
  ): Promise<ParseRecipeResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/parse",
      params,
    );
    return zParseRecipeResponse_unstable.parse(
      raw,
    ) as ParseRecipeResponse_unstable;
  }

  async recipesToYaml_unstable(
    params: RecipeToYamlRequest_unstable,
  ): Promise<RecipeToYamlResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/recipes/to-yaml",
      params,
    );
    return zRecipeToYamlResponse_unstable.parse(
      raw,
    ) as RecipeToYamlResponse_unstable;
  }

  async sessionInfo_unstable(
    params: GetSessionInfoRequest_unstable,
  ): Promise<GetSessionInfoResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/session/info",
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
      "_goose/unstable/session/conversation/truncate",
      params,
    );
  }

  async sessionProjectUpdate_unstable(
    params: UpdateSessionProjectRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/session/project/update", params);
  }

  async sessionRename_unstable(
    params: RenameSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/session/rename", params);
  }

  async sessionArchive_unstable(
    params: ArchiveSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/session/archive", params);
  }

  async sessionUnarchive_unstable(
    params: UnarchiveSessionRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/session/unarchive", params);
  }

  async sourcesCreate_unstable(
    params: CreateSourceRequest_unstable,
  ): Promise<CreateSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/sources/create",
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
      "_goose/unstable/sources/list",
      params,
    );
    return zListSourcesResponse_unstable.parse(
      raw,
    ) as ListSourcesResponse_unstable;
  }

  async sourcesUpdate_unstable(
    params: UpdateSourceRequest_unstable,
  ): Promise<UpdateSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/sources/update",
      params,
    );
    return zUpdateSourceResponse_unstable.parse(
      raw,
    ) as UpdateSourceResponse_unstable;
  }

  async sourcesDelete_unstable(
    params: DeleteSourceRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/sources/delete", params);
  }

  async sourcesExport_unstable(
    params: ExportSourceRequest_unstable,
  ): Promise<ExportSourceResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/sources/export",
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
      "_goose/unstable/sources/import",
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
      "_goose/unstable/dictation/transcribe",
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
      "_goose/unstable/dictation/config",
      params,
    );
    return zDictationConfigResponse_unstable.parse(
      raw,
    ) as DictationConfigResponse_unstable;
  }

  async dictationSecretSave_unstable(
    params: DictationSecretSaveRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod("_goose/unstable/dictation/secret/save", params);
  }

  async dictationSecretDelete_unstable(
    params: DictationSecretDeleteRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/dictation/secret/delete",
      params,
    );
  }

  async dictationModelsList_unstable(
    params: DictationModelsListRequest_unstable,
  ): Promise<DictationModelsListResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/dictation/models/list",
      params,
    );
    return zDictationModelsListResponse_unstable.parse(
      raw,
    ) as DictationModelsListResponse_unstable;
  }

  async dictationModelsDownload_unstable(
    params: DictationModelDownloadRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/dictation/models/download",
      params,
    );
  }

  async dictationModelsDownloadProgress_unstable(
    params: DictationModelDownloadProgressRequest_unstable,
  ): Promise<DictationModelDownloadProgressResponse_unstable> {
    const raw = await this.conn.extMethod(
      "_goose/unstable/dictation/models/download/progress",
      params,
    );
    return zDictationModelDownloadProgressResponse_unstable.parse(
      raw,
    ) as DictationModelDownloadProgressResponse_unstable;
  }

  async dictationModelsCancel_unstable(
    params: DictationModelCancelRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/dictation/models/cancel",
      params,
    );
  }

  async dictationModelsDelete_unstable(
    params: DictationModelDeleteRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/dictation/models/delete",
      params,
    );
  }

  async dictationModelsSelect_unstable(
    params: DictationModelSelectRequest_unstable,
  ): Promise<void> {
    await this.conn.extMethod(
      "_goose/unstable/dictation/models/select",
      params,
    );
  }
}

export interface GooseExtNotifications {
  unstable_sessionUpdate?: (
    notification: GooseSessionNotification_unstable,
  ) => Promise<void>;
}

export interface GooseExtAgentRequests {
  unstable_sessionRecipeRequestParams?: (
    request: RequestRecipeParams_unstable,
  ) => Promise<RecipeParamsResponse_unstable>;
}

export type GooseClientCallbacks = Omit<
  Client,
  "extNotification" | "extMethod"
> &
  Partial<Pick<Client, "extNotification" | "extMethod">> &
  GooseExtNotifications &
  GooseExtAgentRequests;

export function installGooseExtNotificationDispatcher(
  callbacks: GooseClientCallbacks,
): Client {
  const dispatcher: Pick<Client, "extNotification"> = {
    extNotification: async (method, params) => {
      switch (method) {
        case "_goose/unstable/session/update": {
          const parsed = zGooseSessionNotification_unstable.parse(
            params,
          ) as GooseSessionNotification_unstable;
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

export function installGooseExtAgentRequestDispatcher(
  callbacks: GooseClientCallbacks,
): Client {
  const dispatcher: Pick<Client, "extMethod"> = {
    extMethod: async (method, params) => {
      switch (method) {
        case "_goose/unstable/session/recipe/request-params": {
          if (callbacks.unstable_sessionRecipeRequestParams) {
            const parsed = zRequestRecipeParams_unstable.parse(
              params,
            ) as RequestRecipeParams_unstable;
            return await callbacks.unstable_sessionRecipeRequestParams(parsed);
          }
          if (callbacks.extMethod) {
            return await callbacks.extMethod(method, params);
          }
          throw new Error(`unhandled ext method: ${method}`);
        }
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
