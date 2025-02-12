/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type UpsertConfigQuery = {
  /**
   * Whether this configuration value should be treated as a secret
   */
  is_secret?: boolean | null;
  /**
   * The configuration key to upsert
   */
  key: string;
  /**
   * The value to set for the configuration
   */
  value: any;
};
