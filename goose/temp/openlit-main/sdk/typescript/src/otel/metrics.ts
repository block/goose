import {
  MeterProvider,
  PeriodicExportingMetricReader,
  ConsoleMetricExporter,
} from '@opentelemetry/sdk-metrics';
import { metrics } from '@opentelemetry/api';
import SemanticConvention from '../semantic-convention';
import { SetupMetricsOptions } from '../types';
import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-http';

const DB_CLIENT_OPERATION_DURATION_BUCKETS = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5, 10];

const GEN_AI_CLIENT_OPERATION_DURATION_BUCKETS = [
  0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28, 2.56, 5.12, 10.24, 20.48, 40.96, 81.92,
];

const GEN_AI_SERVER_TBT = [0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.75, 1.0, 2.5];

const GEN_AI_SERVER_TFTT = [
  0.001, 0.005, 0.01, 0.02, 0.04, 0.06, 0.08, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];

const GEN_AI_CLIENT_TOKEN_USAGE_BUCKETS = [
  1, 4, 16, 64, 256, 1024, 4096, 16384, 65536, 262144, 1048576, 4194304, 16777216, 67108864,
];

export default class Metrics {
  static meterProvider: MeterProvider;
  static meter: ReturnType<typeof metrics.getMeter>;
  static metricReaders: PeriodicExportingMetricReader[] = [];

  static genaiClientUsageTokens: ReturnType<ReturnType<typeof metrics.getMeter>['createHistogram']>;
  static genaiClientOperationDuration: ReturnType<
    ReturnType<typeof metrics.getMeter>['createHistogram']
  >;
  static genaiServerTbt: ReturnType<ReturnType<typeof metrics.getMeter>['createHistogram']>;
  static genaiServerTtft: ReturnType<ReturnType<typeof metrics.getMeter>['createHistogram']>;
  static dbClientOperationDuration: ReturnType<
    ReturnType<typeof metrics.getMeter>['createHistogram']
  >;
  static genaiRequests: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']>;
  static genaiPromptTokens: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']>;
  static genaiCompletionTokens: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']>;
  static genaiReasoningTokens: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']>;
  static genaiCost: ReturnType<ReturnType<typeof metrics.getMeter>['createHistogram']>;
  static dbRequests: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']>;

  static initializeMetrics() {
    this.genaiClientUsageTokens = this.meter.createHistogram(
      SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
      {
        description: 'Measures number of input and output tokens used',
        unit: '{token}',
        advice: {
          explicitBucketBoundaries: GEN_AI_CLIENT_TOKEN_USAGE_BUCKETS,
        },
      }
    );
    this.genaiClientOperationDuration = this.meter.createHistogram(
      SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
      {
        description: 'GenAI operation duration',
        unit: 's',
        advice: {
          explicitBucketBoundaries: GEN_AI_CLIENT_OPERATION_DURATION_BUCKETS,
        },
      }
    );
    this.genaiServerTbt = this.meter.createHistogram(SemanticConvention.GEN_AI_SERVER_TBT, {
      description: 'Time per output token generated after the first token for successful responses',
      unit: 's',
      advice: {
        explicitBucketBoundaries: GEN_AI_SERVER_TBT,
      },
    });
    this.genaiServerTtft = this.meter.createHistogram(SemanticConvention.GEN_AI_SERVER_TTFT, {
      description: 'Time to generate first token for successful responses',
      unit: 's',
      advice: {
        explicitBucketBoundaries: GEN_AI_SERVER_TFTT,
      },
    });
    this.dbClientOperationDuration = this.meter.createHistogram(
      SemanticConvention.DB_CLIENT_OPERATION_DURATION,
      {
        description: 'DB operation duration',
        unit: 's',
        advice: {
          explicitBucketBoundaries: DB_CLIENT_OPERATION_DURATION_BUCKETS,
        },
      }
    );
    this.genaiRequests = this.meter.createCounter(SemanticConvention.GEN_AI_REQUESTS, {
      description: 'Number of requests to GenAI',
      unit: '1',
    });
    this.genaiPromptTokens = this.meter.createCounter(
      SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
      {
        description: 'Number of prompt tokens processed.',
        unit: '1',
      }
    );
    this.genaiCompletionTokens = this.meter.createCounter(
      SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
      {
        description: 'Number of completion tokens processed.',
        unit: '1',
      }
    );
    this.genaiReasoningTokens = this.meter.createCounter(
      SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS,
      {
        description: 'Number of reasoning thought tokens processed.',
        unit: '1',
      }
    );
    this.genaiCost = this.meter.createHistogram(SemanticConvention.GEN_AI_USAGE_COST, {
      description: 'The distribution of GenAI request costs.',
      unit: 'USD',
    });
    this.dbRequests = this.meter.createCounter(SemanticConvention.DB_REQUESTS, {
      description: 'Number of requests to VectorDBs.',
      unit: '1',
    });
  }

  static handleExporterFallback(err: Error, allowConsoleExporterFallback: boolean): ConsoleMetricExporter {
    if (allowConsoleExporterFallback) {
      console.warn(
        '[Metrics] Falling back to ConsoleMetricExporter. WARNING: ConsoleMetricExporter may expose sensitive data in logs. Do NOT use in production!',
        err
      );
      return new ConsoleMetricExporter();
    } else {
      throw new Error(
        '[Metrics] Failed to initialize OTLPMetricExporter and fallback to ConsoleMetricExporter is disabled. Set allowConsoleExporterFallback=true to enable fallback (not recommended for production).'
      );
    }
  }

  static setup(options: SetupMetricsOptions) {
    if (options.meter) {
      this.meter = options.meter;
      this.initializeMetrics();
      return this.meter;
    }
    try {
      let metricExporter: OTLPMetricExporter | ConsoleMetricExporter;
      try {
        const url = options.otlpEndpoint + '/v1/metrics';
        metricExporter = new OTLPMetricExporter({
          url,
          headers: options.otlpHeaders as Record<string, string> | undefined,
        });
      } catch (err) {
        metricExporter = this.handleExporterFallback(err as Error, options.allowConsoleExporterFallback ?? false);
      }
      const metricReader = new PeriodicExportingMetricReader({
        exportIntervalMillis: options.exportIntervalMillis || 60000,
        exporter: metricExporter,
      });
      this.meterProvider = new MeterProvider({
        resource: options.resource,
        readers: [metricReader],
      });
      metrics.setGlobalMeterProvider(this.meterProvider);
      this.meter = this.meterProvider.getMeter('openlit', '1.0.0');
      this.metricReaders.push(metricReader);
      this.initializeMetrics();
      return this.meter;
    } catch (e) {
      console.error('[Metrics] Failed to initialize metrics:', e);
      return null;
    }
  }
}
