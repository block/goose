import {
  InstrumentationBase,
  InstrumentationModuleDefinition,
  InstrumentationNodeModuleDefinition,
  isWrapped,
} from '@opentelemetry/instrumentation';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
import { INSTRUMENTATION_PREFIX } from '../../constant';
import OllamaWrapper from './wrapper';

export interface OllamaInstrumentationConfig extends InstrumentationConfig {}

export default class OpenlitOllamaInstrumentation extends InstrumentationBase {
  constructor(config: OllamaInstrumentationConfig = {}) {
    super(`${INSTRUMENTATION_PREFIX}/instrumentation-ollama`, '1.0.0', config);
  }

  protected init(): void | InstrumentationModuleDefinition | InstrumentationModuleDefinition[] {
    const module = new InstrumentationNodeModuleDefinition(
      'ollama',
      ['>= 0.5.8'],
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

  public manualPatch(ollama: any): void {
    this._patch(ollama);
  }

  protected _patch(moduleExports: any) {
    try {
      if (isWrapped(moduleExports.Ollama.prototype.chat)) {
        this._unwrap(moduleExports.Ollama.prototype, 'chat');
      }

      this._wrap(moduleExports.Ollama.prototype, 'chat', OllamaWrapper._patchChat(this.tracer));
    } catch (e) {
      console.error('Error in _patch method:', e);
    }
  }

  protected _unpatch(moduleExports: any) {
    this._unwrap(moduleExports.Ollama.prototype, 'chat');
  }
}
