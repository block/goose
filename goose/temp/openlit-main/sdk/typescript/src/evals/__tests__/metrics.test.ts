// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-nocheck

describe('metrics', () => {
  let spy;
  beforeEach(() => {
    jest.resetModules();
    spy = jest.fn();
    // Patch getMeter at runtime
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const otelApi = require('@opentelemetry/api');
    otelApi.metrics.getMeter = () => ({ createCounter: () => ({ add: spy }) });
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const metricsMod = require('../metrics');
    metricsMod.evalCounter = undefined;
  });

  it('recordEvalMetrics calls counter.add with correct attributes', () => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { recordEvalMetrics, EVAL_METRIC_ATTRIBUTES } = require('../metrics');
    const result = {
      verdict: 'yes',
      evaluation: 'Bias',
      score: 0.8,
      classification: 'age',
      explanation: 'reason',
    };
    const validator = 'openai';
    recordEvalMetrics(result, validator);
    expect(spy).toHaveBeenCalledWith(1, expect.objectContaining({
      [EVAL_METRIC_ATTRIBUTES.verdict]: 'yes',
      [EVAL_METRIC_ATTRIBUTES.score]: 0.8,
      [EVAL_METRIC_ATTRIBUTES.validator]: 'openai',
      [EVAL_METRIC_ATTRIBUTES.classification]: 'age',
      [EVAL_METRIC_ATTRIBUTES.explanation]: 'reason',
      evaluation: 'Bias',
    }));
  });

  it('recordEvalMetrics handles missing or undefined result fields', () => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { recordEvalMetrics, EVAL_METRIC_ATTRIBUTES } = require('../metrics');
    const result = {
      verdict: 'no',
      evaluation: 'toxicity',
      score: 0.2,
      // classification and explanation are missing
    };
    const validator = 'anthropic';
    recordEvalMetrics(result, validator);
    expect(spy).toHaveBeenCalledWith(1, expect.objectContaining({
      [EVAL_METRIC_ATTRIBUTES.verdict]: 'no',
      [EVAL_METRIC_ATTRIBUTES.score]: 0.2,
      [EVAL_METRIC_ATTRIBUTES.validator]: 'anthropic',
      [EVAL_METRIC_ATTRIBUTES.classification]: undefined,
      [EVAL_METRIC_ATTRIBUTES.explanation]: undefined,
      evaluation: 'toxicity',
    }));
  });
});
