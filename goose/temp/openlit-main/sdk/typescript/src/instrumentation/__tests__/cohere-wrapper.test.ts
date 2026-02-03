import { Span, trace } from '@opentelemetry/api';
import CohereWrapper from '../cohere/wrapper';
import OpenlitConfig from '../../config';
import OpenLitHelper from '../../helpers';
import BaseWrapper from '../base-wrapper';

jest.mock('../../../src/config');
jest.mock('../../../src/helpers');
jest.mock('../../../src/instrumentation/base-wrapper');

const mockTracer = trace.getTracer('test-tracer');

describe('CohereWrapper', () => {
  let span: Span;

  beforeEach(() => {
    span = mockTracer.startSpan('test-span');
    span.setAttribute = jest.fn();
    jest.clearAllMocks();
  });

  afterEach(() => {
    span.end();
  });

  describe('_chat', () => {
    it('should call recordMetrics after span ends', async () => {
      const mockArgs = [{ message: 'test message' }];
      const mockResponse = { response_id: '123', meta: { billedUnits: { inputTokens: 10, outputTokens: 20 } } };
      const mockGenAIEndpoint = 'cohere.endpoint';

      jest.spyOn(CohereWrapper, '_chatCommonSetter').mockResolvedValue({
        genAIEndpoint: mockGenAIEndpoint,
        model: 'test-model',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'cohere',
      });

      await CohereWrapper._chat({
        args: mockArgs,
        genAIEndpoint: mockGenAIEndpoint,
        response: mockResponse,
        span,
      });

      expect(BaseWrapper.recordMetrics).toHaveBeenCalledWith(span, {
        genAIEndpoint: mockGenAIEndpoint,
        model: 'test-model',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'cohere',
      });
    });
  });

  describe('_chatGenerator', () => {
    it('should call recordMetrics after span ends in generator', async () => {
      const mockArgs = [{ message: 'test message' }];
      const mockResponse = async function* () {
        yield { eventType: 'stream', response: { response_id: '123' } };
        yield { eventType: 'stream-end', response: { response_id: '123', meta: { billedUnits: { inputTokens: 10, outputTokens: 20 } } } };
      }();
      const mockGenAIEndpoint = 'cohere.endpoint';

      jest.spyOn(CohereWrapper, '_chatCommonSetter').mockResolvedValue({
        genAIEndpoint: mockGenAIEndpoint,
        model: 'test-model',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'cohere',
      });

      const generator = CohereWrapper._chatGenerator({
        args: mockArgs,
        genAIEndpoint: mockGenAIEndpoint,
        response: mockResponse,
        span,
      });
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      for await (const _ of generator) {
        // Consume generator without using the value
      }

      expect(BaseWrapper.recordMetrics).toHaveBeenCalledWith(span, {
        genAIEndpoint: mockGenAIEndpoint,
        model: 'test-model',
        user: 'test-user',
        cost: 0.5,
        aiSystem: 'cohere',
      });
    });
  });

  describe('_chatCommonSetter', () => {
    it('should set span attributes and return metric parameters', async () => {
      const mockArgs = [{ message: 'test message', max_tokens: 100, temperature: 0.7 }];
      const mockResult = {
        response_id: '123',
        meta: { billedUnits: { inputTokens: 10, outputTokens: 20 } },
        text: 'response text',
        finishReason: 'stop',
      };
      const mockGenAIEndpoint = 'cohere.endpoint';

      jest.spyOn(OpenlitConfig, 'updatePricingJson').mockResolvedValue({});
      jest.spyOn(OpenLitHelper, 'getChatModelCost').mockReturnValue(0.5);

      const metricParams = await CohereWrapper._chatCommonSetter({
        args: mockArgs,
        genAIEndpoint: mockGenAIEndpoint,
        result: mockResult,
        span,
        stream: false,
      });

      expect(metricParams).toEqual({
        genAIEndpoint: mockGenAIEndpoint,
        model: 'test-model',
        user: "test-user",
        cost: 0.5,
        aiSystem: 'cohere',
      });
    });

    describe('_chatCommonSetter error handling', () => {
      it('should not call recordMetrics and handle the error properly', async () => {
        const mockArgs = [{ message: 'test message', max_tokens: 100, temperature: 0.7 }];
        const mockGenAIEndpoint = 'cohere.endpoint';
        const mockError = new Error('Test error');

        jest.spyOn(CohereWrapper, '_chatCommonSetter').mockRejectedValue(mockError);

        await expect(
          CohereWrapper._chatCommonSetter({
            args: mockArgs,
            genAIEndpoint: mockGenAIEndpoint,
            result: {},
            span,
            stream: false,
          })
        ).rejects.toThrow('Test error');

        expect(BaseWrapper.recordMetrics).not.toHaveBeenCalled();
      });
    });
  });
});
