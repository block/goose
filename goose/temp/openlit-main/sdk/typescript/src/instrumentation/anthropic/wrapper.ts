import { Span, SpanKind, Tracer, context, trace } from '@opentelemetry/api';
import OpenlitConfig from '../../config';
import OpenLitHelper from '../../helpers';
import SemanticConvention from '../../semantic-convention';
import BaseWrapper, { BaseSpanAttributes } from '../base-wrapper';

export default class AnthropicWrapper extends BaseWrapper {
  static aiSystem = SemanticConvention.GEN_AI_SYSTEM_ANTHROPIC;

  static _patchMessageCreate(tracer: Tracer): any {
    const genAIEndpoint = 'anthropic.resources.messages';
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        const { stream = false } = args[0];
        return context
          .with(trace.setSpan(context.active(), span), async () => {
            return originalMethod.apply(this, args);
          })
          .then((response: any) => {
            if (!!stream) {
              return OpenLitHelper.createStreamProxy(
                response,
                AnthropicWrapper._messageCreateGenerator({
                  args,
                  genAIEndpoint,
                  response,
                  span,
                })
              );
            }

            return AnthropicWrapper._messageCreate({ args, genAIEndpoint, response, span });
          })
          .catch((e: any) => {
            OpenLitHelper.handleException(span, e);
            span.end();
            throw e;
          });
      };
    };
  }

  static async _messageCreate({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }) {
    let metricParams: BaseSpanAttributes = {
      genAIEndpoint,
      model: '',
      user: '',
      cost: 0,
      aiSystem: AnthropicWrapper.aiSystem,
    };
    try {
      metricParams = await AnthropicWrapper._messageCreateCommonSetter({
        args,
        genAIEndpoint,
        result: response,
        span,
      });
      return response;
    } catch (e: any) {
      OpenLitHelper.handleException(span, e);
    } finally {
      span.end();
      BaseWrapper.recordMetrics(span, metricParams);
    }
  }

  static async *_messageCreateGenerator({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }) {
    try {
      const result = {
        id: '0',
        model: '',
        stop_reason: '',
        content: [
          {
            text: '',
            role: '',
          },
        ],
        usage: {
          input_tokens: 0,
          output_tokens: 0,
          total_tokens: 0,
        },
      };
      for await (const chunk of response) {
        switch (chunk.type) {
          case 'content_block_delta':
            result.content[0].text += chunk.delta?.text ?? '';
            break;
          case 'message_stop':
            break;

          case 'content_block_stop':
            break;

          case 'message_start':
            if (chunk.message) {
              result.id = chunk.message.id;
              result.model = chunk.message.model;
              result.content[0].role = chunk.message.role;
              result.usage.input_tokens += Number(chunk.message.usage?.input_tokens) ?? 0;
              result.usage.output_tokens += Number(chunk.message.usage?.output_tokens) ?? 0;
              result.stop_reason = chunk.message?.stop_reason ?? '';
            }
            break;

          case 'content_block_start':
            result.content[0].text = chunk.content_block?.text ?? '';
            break;
          case 'message_delta':
            result.stop_reason = chunk.delta?.stop_reason ?? '';
            result.usage.output_tokens += Number(chunk.usage?.output_tokens) ?? 0;
            break;
        }

        yield chunk;
      }

      result.usage.total_tokens = result.usage.output_tokens + result.usage.input_tokens;

      await AnthropicWrapper._messageCreateCommonSetter({
        args,
        genAIEndpoint,
        result,
        span,
      });
      return response;
    } catch (e: any) {
      OpenLitHelper.handleException(span, e);
    } finally {
      span.end();
    }
  }

  static async _messageCreateCommonSetter({
    args,
    genAIEndpoint,
    result,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    result: any;
    span: Span;
  }) {
    const traceContent = OpenlitConfig.traceContent;
    const {
      messages,
      max_tokens = null,
      seed = null,
      temperature = 1,
      top_p,
      top_k,
      user,
      stream = false,
      stop_reason,
    } = args[0];

    // Format 'messages' into a single string
    const messagePrompt = messages || '';
    const formattedMessages = [];

    for (const message of messagePrompt) {
      const role = message.role;
      const content = message.content;

      if (Array.isArray(content)) {
        const contentStr = content
          .map((item) => {
            if ('type' in item) {
              return `${item.type}: ${item.text ? item.text : item.image_url}`;
            } else {
              return `text: ${item.text}`;
            }
          })
          .join(', ');
        formattedMessages.push(`${role}: ${contentStr}`);
      } else {
        formattedMessages.push(`${role}: ${content}`);
      }
    }

    const prompt = formattedMessages.join('\n');
    span.setAttribute(
      SemanticConvention.GEN_AI_OPERATION,
      SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
    );
    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, result.id);

    const model = result.model || 'claude-3-sonnet-20240229';

    const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

    // Calculate cost of the operation
    const cost = OpenLitHelper.getChatModelCost(
      model,
      pricingInfo,
      result.usage.input_tokens,
      result.usage.output_tokens
    );

    AnthropicWrapper.setBaseSpanAttributes(span, {
      genAIEndpoint,
      model,
      user,
      cost,
      aiSystem: AnthropicWrapper.aiSystem,
    });

    // Request Params attributes : Start
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TOP_P, top_p);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TOP_K, top_k);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, max_tokens);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, temperature);

    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, stop_reason);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, stream);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_SEED, seed);
    if (traceContent) {
      span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt);
    }
    // Request Params attributes : End

    span.setAttribute(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, result.usage.input_tokens);
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, result.usage.output_tokens);
    span.setAttribute(
      SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
      result.usage.input_tokens + result.usage.output_tokens
    );

    if (result.stop_reason) {
      span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, result.stop_reason);
    }

    if (traceContent) {
      span.setAttribute(
        SemanticConvention.GEN_AI_CONTENT_COMPLETION,
        result.content?.[0]?.text || ''
      );
    }
    return {
      genAIEndpoint,
      model,
      user,
      cost,
      aiSystem: AnthropicWrapper.aiSystem,
    };
  }
}
