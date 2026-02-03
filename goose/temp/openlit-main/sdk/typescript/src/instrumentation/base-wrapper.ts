import OpenlitConfig from '../config';
import { SDK_NAME, TELEMETRY_SDK_NAME } from '../constant';
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions';
import SemanticConvention from '../semantic-convention';
import { Span, SpanStatusCode } from '@opentelemetry/api';
import Metrics from '../otel/metrics';

export type BaseSpanAttributes = {
  genAIEndpoint: string;
  model: string;
  user?: unknown;
  cost?: number | string;
  aiSystem: string;
};

export default class BaseWrapper {
  static setBaseSpanAttributes(
    span: Span,
    { genAIEndpoint, model, user, cost, aiSystem }: BaseSpanAttributes
  ) {
    const applicationName = OpenlitConfig.applicationName;
    const environment = OpenlitConfig.environment;
    
    if (!applicationName) {
      throw new Error("[Openlit] OpenlitConfig.applicationName is not set. Please check your configuration.");
    }
    if (!environment) {
      throw new Error("[Openlit] OpenlitConfig.environment is not set. Please check your configuration.");
    }

    span.setAttributes({
      [TELEMETRY_SDK_NAME]: SDK_NAME,
    });

    span.setAttribute(TELEMETRY_SDK_NAME, SDK_NAME);
    span.setAttribute(SemanticConvention.GEN_AI_SYSTEM, aiSystem);
    span.setAttribute(SemanticConvention.GEN_AI_ENDPOINT, genAIEndpoint);
    span.setAttribute(SemanticConvention.GEN_AI_ENVIRONMENT, environment);
    span.setAttribute(SemanticConvention.GEN_AI_APPLICATION_NAME, applicationName);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model);
    if (typeof user === 'string' || typeof user === 'number') {
      span.setAttribute(SemanticConvention.GEN_AI_REQUEST_USER, user);
    }
    if (cost !== undefined) {
      span.setAttribute(SemanticConvention.GEN_AI_USAGE_COST, cost);
    }
    span.setStatus({ code: SpanStatusCode.OK });
  }

  static recordMetrics(span: Span, baseAttributes: BaseSpanAttributes) {
    const applicationName = OpenlitConfig.applicationName!;
    const environment = OpenlitConfig.environment!;
    const { genAIEndpoint, model, user, aiSystem, cost } = baseAttributes;

    const inputTokens = BaseWrapper.getSpanAttribute(span, SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS);
    const outputTokens = BaseWrapper.getSpanAttribute(span, SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS);
    const duration = BaseWrapper.getSpanAttribute(span, 'duration') ?? BaseWrapper.getSpanAttribute(span, 'gen_ai.duration');
    const attributes = {
      [ATTR_SERVICE_NAME]: applicationName,
      [SemanticConvention.GEN_AI_SYSTEM]: aiSystem,
      [SemanticConvention.GEN_AI_ENDPOINT]: genAIEndpoint,
      [SemanticConvention.ATTR_DEPLOYMENT_ENVIRONMENT]: environment,
      [SemanticConvention.GEN_AI_REQUEST_MODEL]: model,
      [SemanticConvention.GEN_AI_REQUEST_USER]: typeof user === 'string' || typeof user === 'number' ? user : String(user ?? ''),
    };
    Metrics.genaiRequests?.add(1, attributes);
    if (Number.isFinite(inputTokens)) Metrics.genaiPromptTokens?.add(inputTokens as number, attributes);
    if (Number.isFinite(outputTokens)) Metrics.genaiCompletionTokens?.add(outputTokens as number, attributes);
    if (Number.isFinite(duration)) {
      Metrics.genaiClientOperationDuration?.record((duration as number) / 1e9, attributes);
    }
    const totalTokens = (Number.isFinite(inputTokens) ? inputTokens as number : 0) + (Number.isFinite(outputTokens) ? outputTokens as number : 0);
    if (totalTokens > 0) Metrics.genaiClientUsageTokens?.record(totalTokens, attributes);
    const reasoningTokens = BaseWrapper.getSpanAttribute(span, SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS);
    if (Number.isFinite(reasoningTokens)) Metrics.genaiReasoningTokens?.add(reasoningTokens as number, attributes);
    const tbt = BaseWrapper.getSpanAttribute(span, SemanticConvention.GEN_AI_SERVER_TBT);
    if (Number.isFinite(tbt)) Metrics.genaiServerTbt?.record(tbt as number, attributes);
    const ttft = BaseWrapper.getSpanAttribute(span, SemanticConvention.GEN_AI_SERVER_TTFT);
    if (Number.isFinite(ttft)) Metrics.genaiServerTtft?.record(ttft as number, attributes);
    if (cost !== undefined) {
      const numericCost = typeof cost === 'number' ? cost : Number(cost);
      if (Number.isFinite(numericCost)) {
        Metrics.genaiCost?.record(numericCost, attributes);
      }
    }  
  }

  static getSpanAttribute(span: Span, key: string): number | undefined {
    if (key === 'duration') {
      // Use duration if present, even if 0
      const s = span as { 
        duration?: number; 
        _duration?: number; 
        endTime?: [number, number]; 
        startTime?: [number, number]; 
        attributes?: Record<string, unknown> 
      };
      
      if (s.attributes && typeof s.attributes.duration !== 'undefined') {
        const attrDuration = s.attributes.duration;
        if (typeof attrDuration === 'number' && !isNaN(attrDuration)) {
          return attrDuration;
        }
      }
      
      if (typeof s.duration === 'number' && !isNaN(s.duration)) return s.duration;
      if (typeof s._duration === 'number' && !isNaN(s._duration)) return s._duration;
      
      if (s.endTime && s.startTime) {
        const [endSec, endNano] = s.endTime;
        const [startSec, startNano] = s.startTime;
        const end = endSec * 1e9 + endNano;
        const start = startSec * 1e9 + startNano;
        if (end > start) {
          return end - start;
        }
      }
      return undefined;
    }
    // @ts-expect-error: OpenTelemetry Span may have attributes property in some implementations
    return typeof span.attributes === 'object' ? span.attributes[key] : undefined;
  }
}
