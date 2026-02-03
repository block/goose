import {
  InstrumentationBase,
  InstrumentationModuleDefinition,
  InstrumentationNodeModuleDefinition,
  isWrapped,
} from '@opentelemetry/instrumentation';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
import { INSTRUMENTATION_PREFIX } from '../../constant';
import Cohere from 'cohere-ai';
import CohereWrapper from './wrapper';

export interface CohereInstrumentationConfig extends InstrumentationConfig {}

export default class OpenlitCohereInstrumentation extends InstrumentationBase {
  constructor(config: CohereInstrumentationConfig = {}) {
    super(`${INSTRUMENTATION_PREFIX}/instrumentation-cohere-ai`, '1.0.0', config);
  }

  protected init(): void | InstrumentationModuleDefinition | InstrumentationModuleDefinition[] {
    const module = new InstrumentationNodeModuleDefinition(
      'cohere-ai',
      ['>=7.2.0'],
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

  public manualPatch(cohere: any): void {
    this._patch(cohere);
  }

  protected _patch(moduleExports: typeof Cohere) {
    try {
      if (isWrapped(moduleExports.CohereClient.prototype.embed)) {
        this._unwrap(moduleExports.CohereClient.prototype, 'embed');
      }
      if (isWrapped(moduleExports.CohereClient.prototype.chat)) {
        this._unwrap(moduleExports.CohereClient.prototype, 'chat');
      }
      if (isWrapped(moduleExports.CohereClient.prototype.chatStream)) {
        this._unwrap(moduleExports.CohereClient.prototype, 'chatStream');
      }

      this._wrap(
        moduleExports.CohereClient.prototype,
        'embed',
        CohereWrapper._patchEmbed(this.tracer)
      );

      this._wrap(
        moduleExports.CohereClient.prototype,
        'chat',
        CohereWrapper._patchChat(this.tracer)
      );

      this._wrap(
        moduleExports.CohereClient.prototype,
        'chatStream',
        CohereWrapper._patchChatStream(this.tracer)
      );
    } catch (e) {
      console.error('Error in _patch method:', e);
    }
  }

  protected _unpatch(moduleExports: typeof Cohere) {
    this._unwrap(moduleExports.CohereClient.prototype, 'embed');
    this._unwrap(moduleExports.CohereClient.prototype, 'chat');
    this._unwrap(moduleExports.CohereClient.prototype, 'chatStream');
  }
}
