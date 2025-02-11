/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ConfigKeyQuery } from '../models/ConfigKeyQuery';
import type { ConfigResponse } from '../models/ConfigResponse';
import type { ExtensionQuery } from '../models/ExtensionQuery';
import type { UpsertConfigQuery } from '../models/UpsertConfigQuery';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SuperRoutesConfigManagementService {
  /**
   * @returns ConfigResponse All configuration values retrieved successfully
   * @throws ApiError
   */
  public static readAllConfig(): CancelablePromise<ConfigResponse> {
    return __request(OpenAPI, {
      method: 'GET',
      url: '/config',
    });
  }
  /**
   * @param requestBody
   * @returns string Extension added successfully
   * @throws ApiError
   */
  public static addExtension(requestBody: ExtensionQuery): CancelablePromise<string> {
    return __request(OpenAPI, {
      method: 'POST',
      url: '/config/extension',
      body: requestBody,
      mediaType: 'application/json',
      errors: {
        400: `Invalid request`,
        500: `Internal server error`,
      },
    });
  }
  /**
   * @param requestBody
   * @returns string Extension removed successfully
   * @throws ApiError
   */
  public static removeExtension(requestBody: ConfigKeyQuery): CancelablePromise<string> {
    return __request(OpenAPI, {
      method: 'DELETE',
      url: '/config/extension',
      body: requestBody,
      mediaType: 'application/json',
      errors: {
        404: `Extension not found`,
        500: `Internal server error`,
      },
    });
  }
  /**
   * @param requestBody
   * @returns any Configuration value retrieved successfully
   * @throws ApiError
   */
  public static readConfig(requestBody: ConfigKeyQuery): CancelablePromise<any> {
    return __request(OpenAPI, {
      method: 'GET',
      url: '/config/read',
      body: requestBody,
      mediaType: 'application/json',
      errors: {
        404: `Configuration key not found`,
      },
    });
  }
  /**
   * @param requestBody
   * @returns string Configuration value removed successfully
   * @throws ApiError
   */
  public static removeConfig(requestBody: ConfigKeyQuery): CancelablePromise<string> {
    return __request(OpenAPI, {
      method: 'POST',
      url: '/config/remove',
      body: requestBody,
      mediaType: 'application/json',
      errors: {
        404: `Configuration key not found`,
        500: `Internal server error`,
      },
    });
  }
  /**
   * @param requestBody
   * @returns string Configuration value upserted successfully
   * @throws ApiError
   */
  public static upsertConfig(requestBody: UpsertConfigQuery): CancelablePromise<string> {
    return __request(OpenAPI, {
      method: 'POST',
      url: '/config/upsert',
      body: requestBody,
      mediaType: 'application/json',
      errors: {
        500: `Internal server error`,
      },
    });
  }
}
