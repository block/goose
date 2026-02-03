import { encodingForModel, TiktokenModel } from 'js-tiktoken';
import { Span, SpanStatusCode } from '@opentelemetry/api';

export default class OpenLitHelper {
  static readonly PROMPT_TOKEN_FACTOR = 1000;

  static openaiTokens(text: string, model: string): number {
    try {
      const encoding = encodingForModel(model as TiktokenModel);
      return encoding.encode(text).length;
    } catch (error) {
      console.error(`Error in openaiTokens: ${error}`);
      throw error;
    }
  }

  static generalTokens(text: string): number {
    const encoding = encodingForModel('gpt2');
    return encoding.encode(text).length;
  }

  static getChatModelCost(
    model: string,
    pricingInfo: any,
    promptTokens: number,
    completionTokens: number
  ): number {
    try {
      const cost = (
        (promptTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) * pricingInfo.chat[model].promptPrice +
        (completionTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) *
          pricingInfo.chat[model].completionPrice
      );
      return isNaN(cost) ? 0 : cost;
    } catch (error) {
      console.error(`Error in getChatModelCost: ${error}`);
      return 0;
    }
  }

  static getEmbedModelCost(model: string, pricingInfo: any, promptTokens: number): number {
    try {
      const cost = (promptTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) * pricingInfo.embeddings[model];
      return isNaN(cost) ? 0 : cost;
    } catch (error) {
      console.error(`Error in getEmbedModelCost: ${error}`);
      return 0;
    }
  }

  static getImageModelCost(model: string, pricingInfo: any, size: string, quality: number): number {
    try {
      const cost = pricingInfo.images[model][quality][size];
      return isNaN(cost) ? 0 : cost;
    } catch (error) {
      console.error(`Error in getImageModelCost: ${error}`);
      return 0;
    }
  }

  static getAudioModelCost(model: string, pricingInfo: any, prompt: string): number {
    try {
      const cost = (prompt.length / OpenLitHelper.PROMPT_TOKEN_FACTOR) * pricingInfo.audio[model];
      return isNaN(cost) ? 0 : cost;
    } catch (error) {
      console.error(`Error in getAudioModelCost: ${error}`);
      return 0;
    }
  }

  static async fetchPricingInfo(pricingJson: any) {
    let pricingUrl = 'https://raw.githubusercontent.com/openlit/openlit/main/assets/pricing.json';
    if (pricingJson) {
      let isUrl = false;
      try {
        isUrl = !!new URL(pricingJson);
      } catch {
        isUrl = false;
      }

      if (isUrl) {
        pricingUrl = pricingJson;
      } else {
        try {
          if (typeof pricingJson === 'string') {
            const json = JSON.parse(pricingJson);
            return json;
          } else {
            const json = JSON.parse(JSON.stringify(pricingJson));
            return json;
          }
        } catch {
          return {};
        }
      }
    }

    try {
      const response = await fetch(pricingUrl);
      if (response.ok) {
        return response.json();
      } else {
        throw new Error(`HTTP error occurred while fetching pricing info: ${response.status}`);
      }
    } catch (error) {
      console.error(`Unexpected error occurred while fetching pricing info: ${error}`);
      return {};
    }
  }

  static handleException(span: Span, error: Error): void {
    span.recordException(error);
    span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
  }

  static async createStreamProxy (stream: any, generatorFuncResponse: any): Promise<any> {
    return new Proxy(stream, {
      get (target, prop, receiver) {
        if (prop === Symbol.asyncIterator) {
          return () => generatorFuncResponse
        }
        return Reflect.get(target, prop, receiver)
      }
    })
  }
}
