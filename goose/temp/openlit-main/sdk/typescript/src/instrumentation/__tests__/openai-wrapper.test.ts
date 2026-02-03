import { Span, trace } from '@opentelemetry/api';
import OpenAIWrapper from '../openai/wrapper';
import OpenlitConfig from '../../config';
import OpenLitHelper from '../../helpers';
import BaseWrapper from '../base-wrapper';
import SemanticConvention from '../../semantic-convention';

jest.mock('../../../src/config');
jest.mock('../../../src/helpers');
jest.mock('../../../src/instrumentation/base-wrapper');

const mockTracer = trace.getTracer('test-tracer');

describe('OpenAIWrapper', () => {
  let span: Span;

  beforeEach(() => {
    span = mockTracer.startSpan('test-span');
    span.setAttribute = jest.fn();
    span.addEvent = jest.fn();
    jest.clearAllMocks();
  });

  afterEach(() => {
    span.end();
  });

  describe('_chatCompletion', () => {
    it('should call recordMetrics after span ends', async () => {
      const mockArgs = [{ messages: [{ role: 'user', content: 'test message' }] }];
      const mockResponse = {
        id: '123',
        model: 'gpt-3.5-turbo',
        usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        choices: [
          {
            message: { content: 'response text', role: 'assistant' },
            finish_reason: 'stop',
          },
        ],
      };
      const mockGenAIEndpoint = 'openai.resources.chat.completions';
      jest
        .spyOn(OpenAIWrapper, '_chatCompletionCommonSetter')
        .mockImplementationOnce(async ({ genAIEndpoint, span }) => {
          span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TOP_P, 1);
          span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, 100);
          span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, 0.7);

          return {
            genAIEndpoint,
            model: 'gpt-3.5-turbo',
            user: 'test-user',
            cost: 0.5,
            aiSystem: 'openai',
          };
        });

      await OpenAIWrapper._chatCompletion({
        args: mockArgs,
        genAIEndpoint: mockGenAIEndpoint,
        response: mockResponse,
        span,
      });

      expect(BaseWrapper.recordMetrics).toHaveBeenCalledWith(span, {
        genAIEndpoint: mockGenAIEndpoint,
        model: 'gpt-3.5-turbo',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'openai',
      });
    });
  });

  describe('_chatCompletionCommonSetter', () => {
    it('should set span attributes and return metric parameters', async () => {
      const mockArgs = [
        {
          messages: [{ role: 'user', content: 'test message' }],
          max_tokens: 100,
          temperature: 0.7,
          top_p: 1,
          user: 'test-user',
          presence_penalty: 2,
          frequency_penalty: 3,
          seed: 3,
          stream: false,
          stop: ['STOP'],
        },
      ];

      const mockResult = {
        id: '123',
        model: 'gpt-3.5-turbo',
        usage: { 
          prompt_tokens: 10, 
          completion_tokens: 20, 
          total_tokens: 30,
          completion_tokens_details: { reasoning_tokens: 5 },
        },
        choices: [
          {
            message: { content: 'response text', role: 'assistant' },
            finish_reason: 'stop',
          },
        ],
        system_fingerprint: 'fp_test',
        service_tier: 'default',
      };
      const mockGenAIEndpoint = 'openai.resources.chat.completions';

      jest.restoreAllMocks();

      jest.spyOn(OpenlitConfig, 'updatePricingJson').mockResolvedValue({});
      jest.spyOn(OpenLitHelper, 'getChatModelCost').mockReturnValue(0.5);

      const metricParams = await OpenAIWrapper._chatCompletionCommonSetter({
        args: mockArgs,
        genAIEndpoint: mockGenAIEndpoint,
        result: mockResult,
        span,
      });

      // Basic request parameters
      expect(span.setAttribute).toHaveBeenCalledWith(SemanticConvention.GEN_AI_REQUEST_TOP_P, 1);
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        100
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        0.7
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
        2
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
        3
      );
      expect(span.setAttribute).toHaveBeenCalledWith(SemanticConvention.GEN_AI_REQUEST_SEED, '3');
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_IS_STREAM,
        false
      );
      
      // New attributes
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES,
        ['STOP']
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_RESPONSE_MODEL,
        'gpt-3.5-turbo'
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_RESPONSE_SYSTEM_FINGERPRINT,
        'fp_test'
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER,
        'default'
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
        30
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_REASONING,
        5
      );

      expect(metricParams).toEqual({
        genAIEndpoint: mockGenAIEndpoint,
        model: 'gpt-3.5-turbo',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'openai',
      });
    });

    it('should handle tool calls properly', async () => {
      const mockArgs = [
        {
          messages: [{ role: 'user', content: 'test message' }],
          tools: [{ type: 'function', function: { name: 'get_weather' } }],
        },
      ];

      const mockResult = {
        id: '123',
        model: 'gpt-3.5-turbo',
        usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        choices: [
          {
            message: {
              content: null,
              role: 'assistant',
              tool_calls: [
                {
                  id: 'call_123',
                  type: 'function',
                  function: { name: 'get_weather', arguments: '{"location":"SF"}' },
                },
              ],
            },
            finish_reason: 'tool_calls',
          },
        ],
      };

      jest.spyOn(OpenlitConfig, 'updatePricingJson').mockResolvedValue({});
      jest.spyOn(OpenLitHelper, 'getChatModelCost').mockReturnValue(0.5);

      await OpenAIWrapper._chatCompletionCommonSetter({
        args: mockArgs,
        genAIEndpoint: 'openai.resources.chat.completions',
        result: mockResult,
        span,
      });

      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_TOOL_NAME,
        'get_weather'
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_TOOL_CALL_ID,
        'call_123'
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_TOOL_CALL_ARGUMENTS,
        ['{"location":"SF"}']
      );
      expect(span.setAttribute).toHaveBeenCalledWith(
        SemanticConvention.GEN_AI_TOOL_TYPE,
        'function'
      );
    });
  });
});
