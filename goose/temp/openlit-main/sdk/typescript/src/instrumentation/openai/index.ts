import {
  InstrumentationBase,
  InstrumentationModuleDefinition,
  InstrumentationNodeModuleDefinition,
  isWrapped,
} from '@opentelemetry/instrumentation';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
import { INSTRUMENTATION_PREFIX } from '../../constant';
import OpenAI from 'openai';
import OpenAIWrapper from './wrapper';

export interface OpenAIInstrumentationConfig extends InstrumentationConfig {}

export default class OpenlitOpenAIInstrumentation extends InstrumentationBase {
  constructor(config: OpenAIInstrumentationConfig = {}) {
    super(`${INSTRUMENTATION_PREFIX}/instrumentation-openai`, '1.0.0', config);
  }

  protected init(): void | InstrumentationModuleDefinition | InstrumentationModuleDefinition[] {
    const module = new InstrumentationNodeModuleDefinition(
      'openai',
      ['>=3.1.0 <5'],
      (moduleExports) => {
        this._patch(moduleExports);
        return moduleExports;
      },
      (moduleExports) => {
        if (moduleExports !== undefined) {
          this._unpatch(moduleExports);
        }
      }
    );
    return [module];
  }

  public manualPatch(openai: any): void {
    this._patch(openai);
  }

  protected _patch(moduleExports: typeof OpenAI) {
    try {
      if (isWrapped(moduleExports.OpenAI.Chat.Completions.prototype.create)) {
        this._unwrap(moduleExports.OpenAI.Chat.Completions.prototype, 'create');
      }

      if (isWrapped(moduleExports.OpenAI.Embeddings.prototype.create)) {
        this._unwrap(moduleExports.OpenAI.Embeddings.prototype, 'create');
      }

      if (isWrapped(moduleExports.OpenAI.FineTuning.Jobs.prototype.create)) {
        this._unwrap(moduleExports.OpenAI.FineTuning.Jobs.prototype, 'create');
      }

      if (isWrapped(moduleExports.OpenAI.Images.prototype.generate)) {
        this._unwrap(moduleExports.OpenAI.Images.prototype, 'generate');
      }

      if (isWrapped(moduleExports.OpenAI.Images.prototype.createVariation)) {
        this._unwrap(moduleExports.OpenAI.Images.prototype, 'createVariation');
      }

      if (isWrapped(moduleExports.OpenAI.Audio.Speech.prototype)) {
        this._unwrap(moduleExports.OpenAI.Audio.Speech.prototype, 'create');
      }
      
      // Check if Responses API exists (OpenAI SDK >= 1.92.0)
      if ((moduleExports.OpenAI as any).Responses && isWrapped((moduleExports.OpenAI as any).Responses.prototype.create)) {
        this._unwrap((moduleExports.OpenAI as any).Responses.prototype, 'create');
      }

      this._wrap(
        moduleExports.OpenAI.Chat.Completions.prototype,
        'create',
        OpenAIWrapper._patchChatCompletionCreate(this.tracer)
      );

      this._wrap(
        moduleExports.OpenAI.Embeddings.prototype,
        'create',
        OpenAIWrapper._patchEmbedding(this.tracer)
      );

      this._wrap(
        moduleExports.OpenAI.FineTuning.Jobs.prototype,
        'create',
        OpenAIWrapper._patchFineTune(this.tracer)
      );

      this._wrap(
        moduleExports.OpenAI.Images.prototype,
        'generate',
        OpenAIWrapper._patchImageGenerate(this.tracer)
      );

      this._wrap(
        moduleExports.OpenAI.Images.prototype,
        'createVariation',
        OpenAIWrapper._patchImageVariation(this.tracer)
      );

      this._wrap(
        moduleExports.OpenAI.Audio.Speech.prototype,
        'create',
        OpenAIWrapper._patchAudioCreate(this.tracer)
      );
      
      // Patch Responses API if available (OpenAI SDK >= 1.92.0)
      if ((moduleExports.OpenAI as any).Responses) {
        this._wrap(
          (moduleExports.OpenAI as any).Responses.prototype,
          'create',
          OpenAIWrapper._patchResponsesCreate(this.tracer)
        );
      }
    } catch (e) {
      console.error('Error in _patch method:', e);
    }
  }

  protected _unpatch(moduleExports: typeof OpenAI) {
    this._unwrap(moduleExports.OpenAI.Chat.Completions.prototype, 'create');
    this._unwrap(moduleExports.OpenAI.Embeddings.prototype, 'create');
    this._unwrap(moduleExports.OpenAI.FineTuning.prototype, 'jobs');
    this._unwrap(moduleExports.OpenAI.Images.prototype, 'generate');
    this._unwrap(moduleExports.OpenAI.Images.prototype, 'createVariation');
    this._unwrap(moduleExports.OpenAI.Audio.prototype, 'speech');
    if ((moduleExports.OpenAI as any).Responses) {
      this._unwrap((moduleExports.OpenAI as any).Responses.prototype, 'create');
    }
  }
}
