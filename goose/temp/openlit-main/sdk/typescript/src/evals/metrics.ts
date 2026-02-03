import { EvalsResult } from './types';
import { metrics } from '@opentelemetry/api';

export const EVAL_METRIC_ATTRIBUTES = {
  verdict: 'evals.verdict',
  score: 'evals.score',
  validator: 'evals.validator',
  classification: 'evals.classification',
  explanation: 'evals.explanation',
};

// Global meter and counter for eval metrics
let evalCounter: ReturnType<ReturnType<typeof metrics.getMeter>['createCounter']> | undefined;

function getEvalCounter() {
  if (!evalCounter) {
    // Use the global meter from OpenTelemetry API
    const meter = metrics.getMeter('openlit-evals', '1.0.0');
    evalCounter = meter.createCounter('evals.requests', {
      description: 'Counter for evaluation requests',
      unit: '1',
    });
  }
  return evalCounter;
}

export function recordEvalMetrics(result: EvalsResult, validator: string) {
  const counter = getEvalCounter();
  const attributes = {
    [EVAL_METRIC_ATTRIBUTES.verdict]: result.verdict,
    [EVAL_METRIC_ATTRIBUTES.score]: result.score,
    [EVAL_METRIC_ATTRIBUTES.validator]: validator,
    [EVAL_METRIC_ATTRIBUTES.classification]: result.classification,
    [EVAL_METRIC_ATTRIBUTES.explanation]: result.explanation,
    evaluation: result.evaluation,
  };
  counter.add(1, attributes);
}
