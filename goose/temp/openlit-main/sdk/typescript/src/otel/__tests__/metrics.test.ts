import Metrics from '../metrics';
import SemanticConvention from '../../semantic-convention';
import { defaultResource } from '@opentelemetry/resources';
import { ConsoleMetricExporter } from '@opentelemetry/sdk-metrics';

describe('Metrics creation', () => {
  beforeEach(() => {
    Metrics.setup({ resource: defaultResource(), otlpEndpoint: 'http://localhost:4318'  }); // Ensure metrics are initialized
  });

  it('should create genaiRequests counter and allow increment', () => {
    expect(Metrics.genaiRequests).toBeDefined();
    expect(typeof Metrics.genaiRequests.add).toBe('function');
    expect(() => Metrics.genaiRequests.add(1, { [SemanticConvention.GEN_AI_SYSTEM]: 'openai' })).not.toThrow();
  });

  it('should create genaiPromptTokens counter and allow increment', () => {
    expect(Metrics.genaiPromptTokens).toBeDefined();
    expect(typeof Metrics.genaiPromptTokens.add).toBe('function');
    expect(() => Metrics.genaiPromptTokens.add(42, { [SemanticConvention.GEN_AI_SYSTEM]: 'openai' })).not.toThrow();
  });

  it('should create genaiCompletionTokens counter and allow increment', () => {
    expect(Metrics.genaiCompletionTokens).toBeDefined();
    expect(typeof Metrics.genaiCompletionTokens.add).toBe('function');
    expect(() => Metrics.genaiCompletionTokens.add(24, { [SemanticConvention.GEN_AI_SYSTEM]: 'openai' })).not.toThrow();
  });

  it('should create genaiClientOperationDuration histogram and allow record', () => {
    expect(Metrics.genaiClientOperationDuration).toBeDefined();
    expect(typeof Metrics.genaiClientOperationDuration.record).toBe('function');
    expect(() => Metrics.genaiClientOperationDuration.record(1.23, { [SemanticConvention.GEN_AI_SYSTEM]: 'openai' })).not.toThrow();
  });

  it('should create genaiCost histogram and allow record', () => {
    expect(Metrics.genaiCost).toBeDefined();
    expect(typeof Metrics.genaiCost.record).toBe('function');
    expect(() => Metrics.genaiCost.record(0.99, { [SemanticConvention.GEN_AI_SYSTEM]: 'openai' })).not.toThrow();
  });

  it('should throw an error when allowConsoleExporterFallback is not set and fallback is required', () => {
    expect(() => {
      Metrics.handleExporterFallback(new Error('Simulated OTLPMetricExporter failure'), false);
    }).toThrow('[Metrics] Failed to initialize OTLPMetricExporter and fallback to ConsoleMetricExporter is disabled. Set allowConsoleExporterFallback=true to enable fallback (not recommended for production).');
  });

  it('should use ConsoleMetricExporter when allowConsoleExporterFallback is set to true', () => {
    expect(() => {
      const exporter = Metrics.handleExporterFallback(new Error('Simulated OTLPMetricExporter failure'), true);
      expect(exporter).toBeInstanceOf(ConsoleMetricExporter);
    }).not.toThrow();
  });
});
