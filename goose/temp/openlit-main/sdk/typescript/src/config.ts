import OpenLitHelper from './helpers';
import { OpenlitConfigInterface, PricingObject } from './types';

export default class OpenlitConfig {
  static environment: OpenlitConfigInterface['environment'];
  static applicationName: OpenlitConfigInterface['applicationName'];
  static pricingInfo: PricingObject;
  static tracer: OpenlitConfigInterface['tracer'];
  static otlpEndpoint?: OpenlitConfigInterface['otlpEndpoint'];
  static otlpHeaders?: OpenlitConfigInterface['otlpHeaders'];
  static disableBatch?: OpenlitConfigInterface['disableBatch'];
  static traceContent?: OpenlitConfigInterface['traceContent'];
  static pricing_json?: OpenlitConfigInterface['pricing_json'];

  static async updatePricingJson(pricing_json: any) {
    try {
      const response = await OpenLitHelper.fetchPricingInfo(pricing_json);
      this.pricingInfo = response;
    } catch (e) {
      this.pricingInfo = {};
    }
    return this.pricingInfo;
  }

  static updateConfig({
    environment = 'default',
    applicationName = 'default',
    tracer,
    otlpEndpoint,
    otlpHeaders,
    disableBatch = true,
    traceContent = true,
    pricing_json,
  }: OpenlitConfigInterface) {
    this.environment = environment;
    this.applicationName = applicationName;
    this.tracer = tracer;
    this.otlpEndpoint = otlpEndpoint;
    this.otlpHeaders = otlpHeaders;
    this.disableBatch = disableBatch;
    this.traceContent = traceContent;
    this.pricing_json = pricing_json;
  }
}
