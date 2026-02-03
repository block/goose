import { Span, SpanKind, Tracer, context, trace } from '@opentelemetry/api';
import OpenlitConfig from '../../config';
import OpenLitHelper from '../../helpers';
import SemanticConvention from '../../semantic-convention';
import BaseWrapper, { BaseSpanAttributes } from '../base-wrapper';

class OpenAIWrapper extends BaseWrapper {
  static aiSystem = SemanticConvention.GEN_AI_SYSTEM_OPENAI;
  static _patchChatCompletionCreate(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.chat.completions';
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context
          .with(trace.setSpan(context.active(), span), async () => {
            return originalMethod.apply(this, args);
          })
          .then((response: any) => {
            const { stream = false } = args[0];

            if (stream) {
              return OpenLitHelper.createStreamProxy(
                response,
                OpenAIWrapper._chatCompletionGenerator({
                  args,
                  genAIEndpoint,
                  response,
                  span,
                })
              );
            }

            return OpenAIWrapper._chatCompletion({ args, genAIEndpoint, response, span });
          })
          .catch((e: any) => {
            OpenLitHelper.handleException(span, e);
            span.end();
          });
      };
    };
  }

  static async _chatCompletion({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }): Promise<any> {
    let metricParams;
    try {
      metricParams = await OpenAIWrapper._chatCompletionCommonSetter({
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
      // Record metrics after span has ended if parameters are available
      if (metricParams) {
        BaseWrapper.recordMetrics(span, metricParams);
      }
    }
  }

  static async *_chatCompletionGenerator({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }): AsyncGenerator<unknown, any, unknown> {
    let metricParams;
    const timestamps: number[] = [];
    const startTime = Date.now();
    
    try {
      const { messages } = args[0];
      let { tools } = args[0];
      const result = {
        id: '0',
        created: -1,
        model: '',
        system_fingerprint: '',
        service_tier: 'auto',
        choices: [
          {
            index: 0,
            logprobs: null,
            finish_reason: 'stop',
            message: { role: 'assistant', content: '' },
          },
        ],
        usage: {
          prompt_tokens: 0,
          completion_tokens: 0,
          total_tokens: 0,
          completion_tokens_details: {
            reasoning_tokens: 0,
            audio_tokens: 0,
          },
          prompt_tokens_details: {
            cached_tokens: 0,
            audio_tokens: 0,
          },
        },
      };
      
      let toolCalls: any[] = [];
      
      for await (const chunk of response) {
        timestamps.push(Date.now());
        
        result.id = chunk.id;
        result.created = chunk.created;
        result.model = chunk.model;
        
        if (chunk.system_fingerprint) {
          result.system_fingerprint = chunk.system_fingerprint;
        }
        if (chunk.service_tier) {
          result.service_tier = chunk.service_tier;
        }

        if (chunk.choices[0]?.finish_reason) {
          result.choices[0].finish_reason = chunk.choices[0].finish_reason;
        }
        if (chunk.choices[0]?.logprobs) {
          result.choices[0].logprobs = chunk.choices[0].logprobs;
        }
        if (chunk.choices[0]?.delta.content) {
          result.choices[0].message.content += chunk.choices[0].delta.content;
        }

        // Improved tool calls handling for streaming
        if (chunk.choices[0]?.delta.tool_calls) {
          const deltaTools = chunk.choices[0].delta.tool_calls;
          
          for (const tool of deltaTools) {
            const idx = tool.index || 0;
            
            // Extend array if needed
            while (toolCalls.length <= idx) {
              toolCalls.push({
                id: '',
                type: 'function',
                function: { name: '', arguments: '' }
              });
            }
            
            if (tool.id) {
              // New tool call
              toolCalls[idx].id = tool.id;
              toolCalls[idx].type = tool.type || 'function';
              if (tool.function?.name) {
                toolCalls[idx].function.name = tool.function.name;
              }
              if (tool.function?.arguments) {
                toolCalls[idx].function.arguments = tool.function.arguments;
              }
            } else if (tool.function?.arguments) {
              // Append arguments to existing tool call
              toolCalls[idx].function.arguments += tool.function.arguments;
            }
          }
          
          tools = true;
        }

        yield chunk;
      }
      
      if (toolCalls.length > 0) {
        result.choices[0].message = {
          ...result.choices[0].message,
          tool_calls: toolCalls
        } as any;
      }

      let promptTokens = 0;
      for (const message of messages || []) {
        promptTokens += OpenLitHelper.openaiTokens(message.content as string, result.model) ?? 0;
      }

      const completionTokens = OpenLitHelper.openaiTokens(
        result.choices[0].message.content ?? '',
        result.model
      );
      if (completionTokens) {
        result.usage = {
          prompt_tokens: promptTokens,
          completion_tokens: completionTokens,
          total_tokens: promptTokens + completionTokens,
          completion_tokens_details: result.usage.completion_tokens_details,
          prompt_tokens_details: result.usage.prompt_tokens_details,
        };
      }

      args[0].tools = tools;
      
      // Calculate TTFT and TBT
      const ttft = timestamps.length > 0 ? (timestamps[0] - startTime) / 1000 : 0;
      let tbt = 0;
      if (timestamps.length > 1) {
        const timeDiffs = timestamps.slice(1).map((t, i) => t - timestamps[i]);
        tbt = timeDiffs.reduce((a, b) => a + b, 0) / timeDiffs.length / 1000;
      }

      metricParams = await OpenAIWrapper._chatCompletionCommonSetter({
        args,
        genAIEndpoint,
        result,
        span,
        ttft,
        tbt,
      });

      return result;
    } catch (e: any) {
      OpenLitHelper.handleException(span, e);
    } finally {
      span.end();
      // Record metrics after span has ended if parameters are available
      if (metricParams) {
        BaseWrapper.recordMetrics(span, metricParams);
      }
    }
  }

  static async _chatCompletionCommonSetter({
    args,
    genAIEndpoint,
    result,
    span,
    ttft = 0,
    tbt = 0,
  }: {
    args: any[];
    genAIEndpoint: string;
    result: any;
    span: Span;
    ttft?: number;
    tbt?: number;
  }) {
    const traceContent = OpenlitConfig.traceContent;
    const {
      messages,
      frequency_penalty = 0,
      max_tokens = null,
      n = 1,
      presence_penalty = 0,
      seed = null,
      stop = null,
      temperature = 1,
      top_p,
      user,
      stream = false,
      tools,
    } = args[0];

    // Request Params attributes : Start
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TOP_P, top_p || 1);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, max_tokens || -1);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, temperature);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY, presence_penalty);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, frequency_penalty);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_SEED, seed ? String(seed) : '');
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, stream);
    if (stop) {
      span.setAttribute(SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, Array.isArray(stop) ? stop : [stop]);
    }
    if (user) {
      span.setAttribute(SemanticConvention.GEN_AI_REQUEST_USER, user);
    }

    if (traceContent) {
      // Format 'messages' into a single string
      const messagePrompt = messages || [];
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
      span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt);
    }
    // Request Params attributes : End

    span.setAttribute(
      SemanticConvention.GEN_AI_OPERATION,
      SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
    );

    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, result.id);

    const model = result.model || 'gpt-3.5-turbo';
    const responseModel = result.model || model;

    const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

    // Calculate cost of the operation
    const cost = OpenLitHelper.getChatModelCost(
      model,
      pricingInfo,
      result.usage.prompt_tokens,
      result.usage.completion_tokens
    );

    OpenAIWrapper.setBaseSpanAttributes(span, {
      genAIEndpoint,
      model,
      user,
      cost,
      aiSystem: OpenAIWrapper.aiSystem,
    });

    // Response model
    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_MODEL, responseModel);
    
    // OpenAI-specific attributes
    if (result.system_fingerprint) {
      span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_SYSTEM_FINGERPRINT, result.system_fingerprint);
    }
    if (result.service_tier) {
      span.setAttribute(SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER, result.service_tier);
    }

    // Token usage
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, result.usage.prompt_tokens);
    span.setAttribute(
      SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
      result.usage.completion_tokens
    );
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS, result.usage.total_tokens);
    span.setAttribute(SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, result.usage.total_tokens);
    
    // Enhanced token details
    if (result.usage.completion_tokens_details) {
      if (result.usage.completion_tokens_details.reasoning_tokens) {
        span.setAttribute(
          SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS,
          result.usage.completion_tokens_details.reasoning_tokens
        );
        span.setAttribute(
          SemanticConvention.GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_REASONING,
          result.usage.completion_tokens_details.reasoning_tokens
        );
      }
      if (result.usage.completion_tokens_details.audio_tokens) {
        span.setAttribute(
          SemanticConvention.GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_AUDIO,
          result.usage.completion_tokens_details.audio_tokens
        );
      }
    }
    
    if (result.usage.prompt_tokens_details) {
      if (result.usage.prompt_tokens_details.cached_tokens) {
        span.setAttribute(
          SemanticConvention.GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_READ,
          result.usage.prompt_tokens_details.cached_tokens
        );
      }
      if (result.usage.prompt_tokens_details.audio_tokens) {
        span.setAttribute(
          SemanticConvention.GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_WRITE,
          result.usage.prompt_tokens_details.audio_tokens
        );
      }
    }
    
    // TTFT and TBT metrics
    if (ttft > 0) {
      span.setAttribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft);
    }
    if (tbt > 0) {
      span.setAttribute(SemanticConvention.GEN_AI_SERVER_TBT, tbt);
    }

    // Finish reason
    if (result.choices[0].finish_reason) {
      span.setAttribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON,
        [result.choices[0].finish_reason]
      );
    }
    
    // Output type
    const outputType = typeof result.choices[0].message.content === 'string' 
      ? SemanticConvention.GEN_AI_OUTPUT_TYPE_TEXT 
      : SemanticConvention.GEN_AI_OUTPUT_TYPE_JSON;
    span.setAttribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, outputType);

    // Tool calls handling
    if (result.choices[0].message.tool_calls) {
      const toolCalls = result.choices[0].message.tool_calls;
      const toolNames = toolCalls.map((t: any) => t.function?.name || '').filter(Boolean);
      const toolIds = toolCalls.map((t: any) => t.id || '').filter(Boolean);
      const toolArgs = toolCalls.map((t: any) => t.function?.arguments || '').filter(Boolean);
      const toolTypes = toolCalls.map((t: any) => t.type || '').filter(Boolean);
      
      if (toolNames.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_NAME, toolNames.join(', '));
      }
      if (toolIds.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_CALL_ID, toolIds.join(', '));
      }
      if (toolArgs.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_CALL_ARGUMENTS, toolArgs);
      }
      if (toolTypes.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_TYPE, toolTypes.join(', '));
      }
    }

    // Content
    if (traceContent) {
      // Format completion content - use actual content or empty string if only tool calls
      const completionContent = result.choices[0].message.content || '';
      
      if (n === 1) {
        span.setAttribute(
          SemanticConvention.GEN_AI_CONTENT_COMPLETION,
          completionContent
        );
      } else {
        let i = 0;
        while (i < n) {
          const attribute_name = `${SemanticConvention.GEN_AI_CONTENT_COMPLETION}.${i}`;
          span.setAttribute(attribute_name, result.choices[i].message.content || '');
          i += 1;
        }
      }
      
      // Add events for backward compatibility
      span.addEvent(SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT, {
        [SemanticConvention.GEN_AI_CONTENT_COMPLETION]: completionContent,
      });
    }

    return {
      genAIEndpoint,
      model,
      user,
      cost,
      aiSystem: OpenAIWrapper.aiSystem,
    };
  }

  static _patchEmbedding(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.embeddings';
    const traceContent = OpenlitConfig.traceContent;

    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context.with(trace.setSpan(context.active(), span), async () => {
          let metricParams: BaseSpanAttributes | undefined;
          try {
            const response = await originalMethod.apply(this, args);

            const model = response.model || 'text-embedding-ada-002';
            const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);
            const cost = OpenLitHelper.getEmbedModelCost(
              model,
              pricingInfo,
              response.usage.prompt_tokens
            );

            span.setAttribute(
              SemanticConvention.GEN_AI_OPERATION,
              SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING
            );

            const { dimensions, encoding_format = 'float', input, user } = args[0];
            // Set base span attributes
            OpenAIWrapper.setBaseSpanAttributes(span, {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            });

            // Set missing critical attributes to match Python SDK
            span.setAttribute(SemanticConvention.SERVER_ADDRESS, 'api.openai.com');
            span.setAttribute(SemanticConvention.SERVER_PORT, 443);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, false);
            span.setAttribute(SemanticConvention.GEN_AI_SERVER_TBT, 0);
            span.setAttribute(SemanticConvention.GEN_AI_SERVER_TTFT, 0);
            span.setAttribute(SemanticConvention.GEN_AI_SDK_VERSION, '1.7.0');

            // Request Params attributes : Start
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_ENCODING_FORMATS, [encoding_format]);
            if (dimensions) {
              span.setAttribute(SemanticConvention.GEN_AI_REQUEST_EMBEDDING_DIMENSION, dimensions);
            }
            if (user) {
              span.setAttribute(SemanticConvention.GEN_AI_REQUEST_USER, user);
            }
            if (traceContent) {
              const formattedInput = typeof input === 'string' ? input : JSON.stringify(input);
              span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, formattedInput);
            }

            // Request Params attributes : End
            span.setAttribute(
              SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
              response.usage.prompt_tokens
            );
            span.setAttribute(
              SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
              response.usage.total_tokens
            );
            span.setAttribute(
              SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
              response.usage.prompt_tokens
            );

            metricParams = {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            };

            return response;
          } catch (e: any) {
            OpenLitHelper.handleException(span, e);
            throw e;
          } finally {
            span.end();
            if (metricParams) {
              BaseWrapper.recordMetrics(span, metricParams);
            }
          }
        });
      };
    };
  }

  static _patchFineTune(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.fine_tuning.jobs';

    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context.with(trace.setSpan(context.active(), span), async () => {
          let metricParams;
          try {
            const response = await originalMethod.apply(this, args);

            const model = response.model || 'gpt-3.5-turbo';
            const {
              hyperparameters = {},
              suffix = '',
              training_file,
              user,
              validation_file,
            } = args[0];

            // Set base span attribues
            OpenAIWrapper.setBaseSpanAttributes(span, {
              genAIEndpoint,
              model,
              user,
              aiSystem: OpenAIWrapper.aiSystem,
            });

            span.setAttribute(
              SemanticConvention.GEN_AI_OPERATION,
              SemanticConvention.GEN_AI_OPERATION_TYPE_FINETUNING
            );

            // Request Params attributes : Start

            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TRAINING_FILE, training_file);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_VALIDATION_FILE, validation_file);
            span.setAttribute(
              SemanticConvention.GEN_AI_REQUEST_FINETUNE_BATCH_SIZE,
              hyperparameters?.batch_size || 'auto'
            );
            span.setAttribute(
              SemanticConvention.GEN_AI_REQUEST_FINETUNE_MODEL_LRM,
              hyperparameters?.learning_rate_multiplier || 'auto'
            );
            span.setAttribute(
              SemanticConvention.GEN_AI_REQUEST_FINETUNE_MODEL_EPOCHS,
              hyperparameters?.n_epochs || 'auto'
            );
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_FINETUNE_MODEL_SUFFIX, suffix);
            // Request Params attributes : End

            span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, response.id);
            span.setAttribute(
              SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
              response.usage.prompt_tokens
            );
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_FINETUNE_STATUS, response.status);

            // Store metric parameters for use after span ends
            metricParams = {
              genAIEndpoint,
              model,
              user,
              aiSystem: OpenAIWrapper.aiSystem,
            };

            return response;
          } catch (e: any) {
            OpenLitHelper.handleException(span, e);
          } finally {
            span.end();
            // Record metrics after span has ended if parameters are available
            if (metricParams) {
              BaseWrapper.recordMetrics(span, metricParams);
            }
          }
        });
      };
    };
  }

  static _patchImageGenerate(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.images';
    const traceContent = OpenlitConfig.traceContent;
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context.with(trace.setSpan(context.active(), span), async () => {
          let metricParams;
          try {
            const response = await originalMethod.apply(this, args);

            const {
              prompt,
              quality = 'standard',
              response_format = 'url',
              size = '1024x1024',
              style = 'vivid',
              user,
            } = args[0];

            span.setAttribute(
              SemanticConvention.GEN_AI_OPERATION,
              SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE
            );

            const model = response.model || 'dall-e-2';

            const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

            // Calculate cost of the operation
            const cost =
              (response.data?.length || 1) *
              OpenLitHelper.getImageModelCost(model, pricingInfo, size, quality);

            OpenAIWrapper.setBaseSpanAttributes(span, {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            });

            // Request Params attributes : Start
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_SIZE, size);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_QUALITY, quality);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_STYLE, style);

            if (traceContent) {
              span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt);
            }
            // Request Params attributes : End

            let imagesCount = 0;

            if (response.data) {
              for (const items of response.data) {
                span.setAttribute(
                  `${SemanticConvention.GEN_AI_CONTENT_REVISED_PROMPT}.${imagesCount}`,
                  items.revised_prompt || ''
                );

                const attributeName = `${SemanticConvention.GEN_AI_RESPONSE_IMAGE}.${imagesCount}`;
                span.setAttribute(attributeName, items[response_format]);

                imagesCount++;
              }
            }

            // Store metric parameters for use after span ends
            metricParams = {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            };

            return response;
          } catch (e: any) {
            OpenLitHelper.handleException(span, e);
          } finally {
            span.end();
            // Record metrics after span has ended if parameters are available
            if (metricParams) {
              BaseWrapper.recordMetrics(span, metricParams);
            }
          }
        });
      };
    };
  }

  static _patchImageVariation(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.images';
    const traceContent = OpenlitConfig.traceContent;
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context.with(trace.setSpan(context.active(), span), async () => {
          let metricParams;
          try {
            const response = await originalMethod.apply(this, args);

            const {
              prompt,
              quality = 'standard',
              response_format = 'url',
              size = '1024x1024',
              style = 'vivid',
              user,
            } = args[0];

            span.setAttribute(
              SemanticConvention.GEN_AI_OPERATION,
              SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE
            );
            span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, response.created);

            const model = response.model || 'dall-e-2';

            const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

            // Calculate cost of the operation
            const cost =
              (response.data?.length || 1) *
              OpenLitHelper.getImageModelCost(model, pricingInfo, size, quality);

            OpenAIWrapper.setBaseSpanAttributes(span, {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            });

            // Request Params attributes : Start
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_SIZE, size);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_QUALITY, quality);
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_STYLE, style);

            if (traceContent) {
              span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt);
            }
            // Request Params attributes : End

            let imagesCount = 0;
            if (response.data) {
              for (const items of response.data) {
                span.setAttribute(
                  `${SemanticConvention.GEN_AI_CONTENT_REVISED_PROMPT}.${imagesCount}`,
                  items.revised_prompt || ''
                );

                const attributeName = `${SemanticConvention.GEN_AI_RESPONSE_IMAGE}.${imagesCount}`;
                span.setAttribute(attributeName, items[response_format]);

                imagesCount++;
              }
            }

            // Store metric parameters for use after span ends
            metricParams = {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            };

            return response;
          } catch (e: any) {
            OpenLitHelper.handleException(span, e);
          } finally {
            span.end();
            // Record metrics after span has ended if parameters are available
            if (metricParams) {
              BaseWrapper.recordMetrics(span, metricParams);
            }
          }
        });
      };
    };
  }

  static _patchAudioCreate(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.audio.speech';
    const traceContent = OpenlitConfig.traceContent;
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context.with(trace.setSpan(context.active(), span), async () => {
          let metricParams;
          try {
            const response = await originalMethod.apply(this, args);

            const { input, user, voice, response_format = 'mp3', speed = 1 } = args[0];

            span.setAttribute(
              SemanticConvention.GEN_AI_OPERATION,
              SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO
            );

            const model = response.model || 'tts-1';

            const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

            // Calculate cost of the operation
            const cost = OpenLitHelper.getAudioModelCost(model, pricingInfo, input);

            OpenAIWrapper.setBaseSpanAttributes(span, {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            });

            // Request Params attributes : Start
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_AUDIO_VOICE, voice);
            span.setAttribute(
              SemanticConvention.GEN_AI_REQUEST_AUDIO_RESPONSE_FORMAT,
              response_format
            );
            span.setAttribute(SemanticConvention.GEN_AI_REQUEST_AUDIO_SPEED, speed);

            if (traceContent) {
              span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, input);
            }
            // Request Params attributes : End

            // Store metric parameters for use after span ends
            metricParams = {
              genAIEndpoint,
              model,
              user,
              cost,
              aiSystem: OpenAIWrapper.aiSystem,
            };

            return response;
          } catch (e: any) {
            OpenLitHelper.handleException(span, e);
          } finally {
            span.end();
            // Record metrics after span has ended if parameters are available
            if (metricParams) {
              BaseWrapper.recordMetrics(span, metricParams);
            }
          }
        });
      };
    };
  }

  static _patchResponsesCreate(tracer: Tracer): any {
    const genAIEndpoint = 'openai.resources.responses';
    return (originalMethod: (...args: any[]) => any) => {
      return async function (this: any, ...args: any[]) {
        const span = tracer.startSpan(genAIEndpoint, { kind: SpanKind.CLIENT });
        return context
          .with(trace.setSpan(context.active(), span), async () => {
            return originalMethod.apply(this, args);
          })
          .then((response: any) => {
            const { stream = false } = args[0];

            if (stream) {
              return OpenLitHelper.createStreamProxy(
                response,
                OpenAIWrapper._responsesGenerator({
                  args,
                  genAIEndpoint,
                  response,
                  span,
                })
              );
            }

            return OpenAIWrapper._responsesComplete({ args, genAIEndpoint, response, span });
          })
          .catch((e: any) => {
            OpenLitHelper.handleException(span, e);
            span.end();
          });
      };
    };
  }

  static async _responsesComplete({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }): Promise<any> {
    let metricParams;
    try {
      metricParams = await OpenAIWrapper._responsesCommonSetter({
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
      if (metricParams) {
        BaseWrapper.recordMetrics(span, metricParams);
      }
    }
  }

  static async *_responsesGenerator({
    args,
    genAIEndpoint,
    response,
    span,
  }: {
    args: any[];
    genAIEndpoint: string;
    response: any;
    span: Span;
  }): AsyncGenerator<unknown, any, unknown> {
    let metricParams;
    const timestamps: number[] = [];
    const startTime = Date.now();
    
    try {
      const { input } = args[0];
      const result = {
        id: '',
        model: '',
        service_tier: 'default',
        status: 'completed',
        output: [] as any[],
        usage: {
          input_tokens: 0,
          output_tokens: 0,
          output_tokens_details: {
            reasoning_tokens: 0,
          },
        },
      };
      
      let llmResponse = '';
      let responseTools: any[] = [];
      
      for await (const chunk of response) {
        timestamps.push(Date.now());
        
        if (chunk.type === 'response.output_text.delta') {
          llmResponse += chunk.delta || '';
        } else if (chunk.type === 'response.output_item.added') {
          const item = chunk.item;
          if (item?.type === 'function_call') {
            responseTools.push({
              id: item.id,
              call_id: item.call_id,
              name: item.name,
              type: item.type,
              arguments: item.arguments || '',
              status: item.status,
            });
          }
        } else if (chunk.type === 'response.function_call_arguments.delta') {
          const itemId = chunk.item_id;
          const delta = chunk.delta || '';
          const tool = responseTools.find(t => t.id === itemId);
          if (tool) {
            tool.arguments += delta;
          }
        } else if (chunk.type === 'response.completed') {
          const responseData = chunk.response;
          result.id = responseData.id;
          result.model = responseData.model;
          result.status = responseData.status;
          
          const usage = responseData.usage || {};
          result.usage.input_tokens = usage.input_tokens || 0;
          result.usage.output_tokens = usage.output_tokens || 0;
          result.usage.output_tokens_details.reasoning_tokens = 
            usage.output_tokens_details?.reasoning_tokens || 0;
        }

        yield chunk;
      }
      
      // Construct output array
      if (llmResponse) {
        result.output.push({
          type: 'message',
          content: [{ type: 'text', text: llmResponse }],
        });
      }
      
      if (responseTools.length > 0) {
        result.output.push(...responseTools);
      }
      
      // Calculate TTFT and TBT
      const ttft = timestamps.length > 0 ? (timestamps[0] - startTime) / 1000 : 0;
      let tbt = 0;
      if (timestamps.length > 1) {
        const timeDiffs = timestamps.slice(1).map((t, i) => t - timestamps[i]);
        tbt = timeDiffs.reduce((a, b) => a + b, 0) / timeDiffs.length / 1000;
      }

      metricParams = await OpenAIWrapper._responsesCommonSetter({
        args,
        genAIEndpoint,
        result,
        span,
        ttft,
        tbt,
      });

      return result;
    } catch (e: any) {
      OpenLitHelper.handleException(span, e);
    } finally {
      span.end();
      if (metricParams) {
        BaseWrapper.recordMetrics(span, metricParams);
      }
    }
  }

  static async _responsesCommonSetter({
    args,
    genAIEndpoint,
    result,
    span,
    ttft = 0,
    tbt = 0,
  }: {
    args: any[];
    genAIEndpoint: string;
    result: any;
    span: Span;
    ttft?: number;
    tbt?: number;
  }) {
    const traceContent = OpenlitConfig.traceContent;
    const {
      input,
      temperature = 1.0,
      top_p = 1.0,
      max_output_tokens,
      reasoning,
      stream = false,
    } = args[0];

    // Format input for prompt
    let prompt = '';
    if (typeof input === 'string') {
      prompt = input;
    } else if (Array.isArray(input)) {
      const formattedMessages = [];
      for (const item of input) {
        const role = item.role || 'user';
        const content = item.content;
        
        if (typeof content === 'string') {
          formattedMessages.push(`${role}: ${content}`);
        } else if (Array.isArray(content)) {
          const contentParts = content
            .map((part: any) => {
              if (part.type === 'input_text') {
                return `text: ${part.text || ''}`;
              } else if (part.type === 'input_image' && part.image_url && !part.image_url.startsWith('data:')) {
                return `image_url: ${part.image_url}`;
              }
              return '';
            })
            .filter(Boolean)
            .join(', ');
          formattedMessages.push(`${role}: ${contentParts}`);
        }
      }
      prompt = formattedMessages.join('\n');
    }

    // Request Params attributes
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, temperature);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_TOP_P, top_p);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, max_output_tokens || -1);
    span.setAttribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, stream);
    
    if (reasoning?.effort) {
      span.setAttribute('gen_ai.request.reasoning_effort', reasoning.effort);
    }

    if (traceContent) {
      span.setAttribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt);
    }

    span.setAttribute(
      SemanticConvention.GEN_AI_OPERATION,
      SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
    );

    const model = result.model || 'gpt-4o';
    const responseModel = result.model || model;

    const pricingInfo = await OpenlitConfig.updatePricingJson(OpenlitConfig.pricing_json);

    // Calculate cost
    const inputTokens = result.usage?.input_tokens || 0;
    const outputTokens = result.usage?.output_tokens || 0;
    const cost = OpenLitHelper.getChatModelCost(
      model,
      pricingInfo,
      inputTokens,
      outputTokens
    );

    OpenAIWrapper.setBaseSpanAttributes(span, {
      genAIEndpoint,
      model,
      user: '',
      cost,
      aiSystem: OpenAIWrapper.aiSystem,
    });

    // Response attributes
    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_ID, result.id);
    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_MODEL, responseModel);
    span.setAttribute(SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [result.status || 'completed']);
    span.setAttribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, SemanticConvention.GEN_AI_OUTPUT_TYPE_TEXT);
    
    if (result.service_tier) {
      span.setAttribute(SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER, result.service_tier);
    }

    // Token usage
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, inputTokens);
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, outputTokens);
    span.setAttribute(SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS, inputTokens + outputTokens);
    span.setAttribute(SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, inputTokens + outputTokens);
    
    // Reasoning tokens
    if (result.usage?.output_tokens_details?.reasoning_tokens) {
      span.setAttribute(
        SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS,
        result.usage.output_tokens_details.reasoning_tokens
      );
    }
    
    // TTFT and TBT metrics
    if (ttft > 0) {
      span.setAttribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft);
    }
    if (tbt > 0) {
      span.setAttribute(SemanticConvention.GEN_AI_SERVER_TBT, tbt);
    }

    // Extract completion text from output
    let completionText = '';
    if (result.output && Array.isArray(result.output)) {
      for (const item of result.output) {
        if (item.type === 'message' && item.content) {
          for (const content of item.content) {
            if (content.type === 'text' || content.type === 'output_text') {
              completionText += content.text || '';
            }
          }
        }
      }
    }

    // Tool calls handling for Responses API
    const toolCalls = result.tools || [];
    if (toolCalls.length > 0) {
      const toolNames = toolCalls.map((t: any) => t.name || '').filter(Boolean);
      const toolIds = toolCalls.map((t: any) => t.call_id || '').filter(Boolean);
      const toolArgs = toolCalls.map((t: any) => t.arguments || '').filter(Boolean);
      const toolTypes = toolCalls.map((t: any) => t.type || '').filter(Boolean);
      
      if (toolNames.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_NAME, toolNames.join(', '));
      }
      if (toolIds.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_CALL_ID, toolIds.join(', '));
      }
      if (toolArgs.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_CALL_ARGUMENTS, toolArgs.join(', '));
      }
      if (toolTypes.length > 0) {
        span.setAttribute(SemanticConvention.GEN_AI_TOOL_TYPE, toolTypes.join(', '));
      }
    }

    // Content
    if (traceContent) {
      // Set completion content - use actual text or empty string if only tool calls
      span.setAttribute(SemanticConvention.GEN_AI_CONTENT_COMPLETION, completionText);
      
      // Add events for backward compatibility
      span.addEvent(SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT, {
        [SemanticConvention.GEN_AI_CONTENT_COMPLETION]: completionText,
      });
    }

    return {
      genAIEndpoint,
      model,
      user: '',
      cost,
      aiSystem: OpenAIWrapper.aiSystem,
    };
  }
}

export default OpenAIWrapper;
