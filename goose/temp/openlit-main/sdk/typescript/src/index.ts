import { resourceFromAttributes } from '@opentelemetry/resources';
import { ATTR_SERVICE_NAME, ATTR_TELEMETRY_SDK_NAME } from '@opentelemetry/semantic-conventions';
import { NodeSDK } from '@opentelemetry/sdk-node';
import { OpenlitOptions } from './types';
import Tracing from './otel/tracing';
import { DEFAULT_APPLICATION_NAME, DEFAULT_ENVIRONMENT, SDK_NAME } from './constant';
import { SpanExporter } from '@opentelemetry/sdk-trace-base';
import BaseOpenlit from './features/base';
import { Hallucination, Bias, Toxicity, All } from './evals';
import Metrics from './otel/metrics';
import SemanticConvention from './semantic-convention';
import { PromptInjection } from './guard/prompt-injection';
import { SensitiveTopic } from './guard/sensitive-topic';
import { TopicRestriction } from './guard/topic-restriction';
import { All as GuardAll } from './guard/all';

// Factory functions for evals
const evals = {
  Hallucination: (options: ConstructorParameters<typeof Hallucination>[0]) =>
    new Hallucination(options),
  Bias: (options: ConstructorParameters<typeof Bias>[0]) => new Bias(options),
  Toxicity: (options: ConstructorParameters<typeof Toxicity>[0]) => new Toxicity(options),
  All: (options: ConstructorParameters<typeof All>[0]) => new All(options),
};

// Factory functions for guards
const guard = {
  PromptInjection: (options: ConstructorParameters<typeof PromptInjection>[0]) => new PromptInjection(options),
  SensitiveTopic: (options: ConstructorParameters<typeof SensitiveTopic>[0]) => new SensitiveTopic(options),
  TopicRestriction: (options: ConstructorParameters<typeof TopicRestriction>[0]) => new TopicRestriction(options),
  All: (options: ConstructorParameters<typeof GuardAll>[0]) => new GuardAll(options),
};

class Openlit extends BaseOpenlit {
  static resource: ReturnType<typeof resourceFromAttributes>;
  static options: OpenlitOptions;
  static _sdk: NodeSDK;
  static evals = evals;
  static guard = guard;
  static init(options?: OpenlitOptions) {
    try {
      const { environment = DEFAULT_ENVIRONMENT, applicationName = DEFAULT_APPLICATION_NAME } =
        options || {};

      const otlpEndpoint =
        (options?.otlpEndpoint || process.env.OTEL_EXPORTER_OTLP_ENDPOINT || "http://localhost:4318").replace(/\/v1\/traces$/, '');

      let otlpHeaders = options?.otlpHeaders;
      if (!otlpHeaders) {
        if (process.env.OTEL_EXPORTER_OTLP_HEADERS) {
          otlpHeaders = process.env.OTEL_EXPORTER_OTLP_HEADERS.split(',').reduce(
            (acc: Record<string, string>, items: string) => {
              const keyVal: string[] = items.split('=');
              acc[keyVal[0]] = keyVal[1];
              return acc;
            },
            {} as Record<string, string>
          );
        } else {
          otlpHeaders = {};
        }
      }

      this.options = options || {};
      this.options.otlpEndpoint = otlpEndpoint;
      this.options.otlpHeaders = otlpHeaders;
      this.options.disableBatch =
        options?.disableBatch === undefined ? true : !!options.disableBatch;

      this.resource = resourceFromAttributes({
        [ATTR_SERVICE_NAME]: applicationName,
        [SemanticConvention.ATTR_DEPLOYMENT_ENVIRONMENT]: environment,
        [ATTR_TELEMETRY_SDK_NAME]: SDK_NAME,
      });

      Tracing.setup({
        ...this.options,
        environment,
        applicationName,
        otlpEndpoint,
        otlpHeaders,
        resource: this.resource,
      });
      const exportIntervalMillis =
        Number(process.env.OTEL_EXPORTER_OTLP_METRICS_EXPORT_INTERVAL ?? 60000) || 60000;

      Metrics.setup({
        ...options,
        environment,
        applicationName,
        otlpEndpoint,
        otlpHeaders,
        resource: this.resource,
        exportIntervalMillis: exportIntervalMillis,
      });

      this._sdk = new NodeSDK({
        resource: this.resource,
        traceExporter: Tracing.traceExporter as SpanExporter,
        metricReader: Metrics.metricReaders[0],
      });

      // This was causing the traceProvider initilization with multiple instances.
      // this._sdk.start();
    } catch (e) {
      console.log('Connection time out', e);
    }
  }
}

const openlit = Openlit as typeof Openlit & {
  evals: typeof evals;
  guard: typeof guard;
};

export default openlit;
export { Openlit };
export type { OpenlitOptions } from './types';
