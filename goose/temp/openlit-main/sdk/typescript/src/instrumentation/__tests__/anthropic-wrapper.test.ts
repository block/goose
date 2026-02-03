import { Span, trace } from '@opentelemetry/api';
import AnthropicWrapper from '../anthropic/wrapper';
import OpenlitConfig from '../../config';
import OpenLitHelper from '../../helpers';
import BaseWrapper from '../base-wrapper';
import SemanticConvention from '../../semantic-convention';

jest.mock('../../../src/config');
jest.mock('../../../src/helpers');
jest.mock('../../../src/instrumentation/base-wrapper');

const mockTracer = trace.getTracer('test-tracer');

describe('AnthropicWrapper', () => {
  let span: Span;

  beforeEach(() => {
    span = mockTracer.startSpan('test-span');
    span.setAttribute = jest.fn();
    jest.clearAllMocks();
  });

  afterEach(() => {
    span.end();
  });

  describe('_messageCreate', () => {
    it('should call recordMetrics after span ends', async () => {
      const mockArgs = [{ message: 'test message' }];
      const mockResponse = { response_id: '123', meta: { billedUnits: { inputTokens: 10, outputTokens: 20 } } };

      jest.spyOn(BaseWrapper, 'recordMetrics').mockImplementation(() => {});

      jest.spyOn(AnthropicWrapper, '_messageCreateCommonSetter').mockImplementationOnce(async ({ genAIEndpoint, span }) => {
        span.setAttribute(SemanticConvention.GEN_AI_OPERATION, SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT);
        span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, '123');
        span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, 100);
        span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, 0.7);
        span.setAttribute(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, 10);
        span.setAttribute(SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, 20);
        span.setAttribute(SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS, 30);
        span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, 'stop');

        return {
          genAIEndpoint,
          model: 'test-model',
          user: 'test-user',
          cost: 0.5,
          aiSystem: 'anthropic',
        };
      });

      await AnthropicWrapper._messageCreate({
        args: mockArgs,
        genAIEndpoint: 'anthropic.endpoint',
        response: mockResponse,
        span,
      });

      expect(BaseWrapper.recordMetrics).toHaveBeenCalledWith(span, {
        model: 'test-model',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'anthropic',
        genAIEndpoint: 'anthropic.endpoint',
      });
    });
  });

  describe('_messageCommonSetter', () => {
    it('should set span attributes and return metric parameters', async () => {
      const mockArgs = [{ message: 'test message', max_tokens: 100, temperature: 0.7 }];
      const mockResult = {
        id: '123',
        usage: { input_tokens: 10, output_tokens: 20 },
        model: 'claude-3-sonnet-20240229',
        stop_reason: 'stop',
      };

      jest.spyOn(OpenlitConfig, 'updatePricingJson').mockResolvedValue({});
      jest.spyOn(OpenLitHelper, 'getChatModelCost').mockReturnValue(0.5);

      const setAttributeSpy = jest.spyOn(span, 'setAttribute');

      await AnthropicWrapper._messageCreateCommonSetter({
        args: mockArgs,
        genAIEndpoint: 'anthropic.endpoint',
        result: mockResult,
        span,
      });

      expect(setAttributeSpy).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_OPERATION,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
      );
      expect(setAttributeSpy).toHaveBeenCalledWith(SemanticConvention.GEN_AI_RESPONSE_ID, '123');
      expect(setAttributeSpy).toHaveBeenCalledWith(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, 100);
      expect(setAttributeSpy).toHaveBeenCalledWith(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, 0.7);
      expect(setAttributeSpy).toHaveBeenCalledWith(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, 10);
      expect(setAttributeSpy).toHaveBeenCalledWith(SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, 20);
      expect(setAttributeSpy).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
        30
      );
      expect(setAttributeSpy).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON,
        'stop'
      );
    });
  });
});
