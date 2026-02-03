import { SetupTracerOptions } from '../types';
import {
  NodeTracerProvider,
  BatchSpanProcessor,
  ConsoleSpanExporter,
  SimpleSpanProcessor,
  SpanExporter,
} from '@opentelemetry/sdk-trace-node';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-http';

import Instrumentations from '../instrumentation';
import OpenlitConfig from '../config';

export default class Tracing {
  static traceProvider: NodeTracerProvider;
  static traceExporter: OTLPTraceExporter;
  static async setup(options: SetupTracerOptions) {
    if (options.tracer) return options.tracer;
    try {
      const consoleSpanExporter = new ConsoleSpanExporter();
      const url = options.otlpEndpoint + "/v1/traces";
      const otlpTraceExporter = new OTLPTraceExporter({
        url,
        headers: options.otlpHeaders as Record<string, string> | undefined,
      });

      const spanProcessors = [
        new SimpleSpanProcessor(consoleSpanExporter),
        options.disableBatch
          ? new SimpleSpanProcessor(otlpTraceExporter as SpanExporter)
          : new BatchSpanProcessor(otlpTraceExporter as SpanExporter),
      ];

      this.traceProvider = new NodeTracerProvider({
        resource: options.resource,
        spanProcessors,
      });

      OpenlitConfig.updateConfig({
        ...options,
        tracer: options.tracer || Tracing.traceProvider,
      });

      Instrumentations.setup(
        Tracing.traceProvider,
        options?.disabledInstrumentations,
        options?.instrumentations
      );

      this.traceExporter = otlpTraceExporter;

      this.traceProvider.register();
    } catch (e) {
      console.error('[Traces] Failed to initialize traces:', e);
      return null;
    }
  }
}
