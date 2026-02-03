// Utility functions for guardrails (regex, LLM, metrics)
import { GuardResult, CustomRule } from './types';

export function customRuleDetection(text: string, customRules: CustomRule[] = []): GuardResult {
  for (const rule of customRules) {
    let regex: RegExp;
    try {
      regex = new RegExp(rule.pattern, 'i');
    } catch (e) {
      // eslint-disable-next-line no-console
      console.warn(`Invalid regex pattern in customRuleDetection: "${rule.pattern}". Skipping rule.`, e);
      continue;
    }
    if (regex.test(text)) {
      return {
        verdict: rule.verdict || 'yes',
        guard: rule.guard || 'custom',
        score: rule.score ?? 0.5,
        classification: rule.classification,
        explanation: rule.explanation || 'Matched custom rule pattern.'
      };
    }
  }
  return {
    score: 0,
    verdict: 'none',
    guard: 'none',
    classification: 'none',
    explanation: 'none'
  };
}

// Metric helpers (OpenTelemetry)
import { metrics } from '@opentelemetry/api';

export function guardMetrics() {
  const meter = metrics.getMeter('openlit.guard', '0.1.0');
  const guardRequests = meter.createCounter('openlit.guard.requests', {
    description: 'Counter for Guard requests',
    unit: '1'
  });
  return guardRequests;
}

export function guardMetricAttributes(verdict: string, score: number, validator: string, classification: string, explanation: string) {
  return {
    'telemetry.sdk.name': 'openlit',
    'openlit.guard.verdict': verdict,
    'openlit.guard.score': score,
    'openlit.guard.validator': validator,
    'openlit.guard.classification': classification,
    'openlit.guard.explanation': explanation,
  };
}

export function toGuardResult(result: unknown, guardType: string): GuardResult {
  const r = result as Partial<GuardResult & { [key: string]: unknown }>;
  return {
    score: typeof r.score === 'number' ? r.score : 0,
    verdict: typeof r.verdict === 'string' ? (r.verdict as GuardResult['verdict']) : 'none',
    guard: typeof r.guard === 'string' ? r.guard : guardType,
    classification: typeof r.classification === 'string' ? r.classification : 'none',
    explanation: typeof r.explanation === 'string' ? r.explanation : 'none',
  };
}

export function applyThresholdScore(result: GuardResult, threshold: number): GuardResult {
  if (result.score < threshold) {
    return {
      score: 0,
      verdict: 'none',
      guard: 'none',
      classification: 'none',
      explanation: 'none'
    };
  }
  return result;
}
