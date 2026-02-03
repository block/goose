import {
  InstrumentationBase,
  InstrumentationModuleDefinition,
  InstrumentationNodeModuleDefinition,
  isWrapped,
} from '@opentelemetry/instrumentation';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
import { INSTRUMENTATION_PREFIX } from '../../constant';
import Anthropic from '@anthropic-ai/sdk';
import AnthropicWrapper from './wrapper';

export interface AnthropicInstrumentationConfig extends InstrumentationConfig {}

export default class OpenlitAnthropicInstrumentation extends InstrumentationBase {
  constructor(config: AnthropicInstrumentationConfig = {}) {
    super(`${INSTRUMENTATION_PREFIX}/instrumentation-anthropic`, '1.0.0', config);
  }

  protected init(): void | InstrumentationModuleDefinition | InstrumentationModuleDefinition[] {
    const module = new InstrumentationNodeModuleDefinition(
      '@anthropic-ai/sdk',
      ['>= 0.21.0'],
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

  public manualPatch(anthropic: any): void {
    this._patch(anthropic);
  }

  protected _patch(moduleExports: typeof Anthropic) {
    try {
      if (isWrapped(moduleExports.Anthropic.Messages.prototype.create)) {
        this._unwrap(moduleExports.Anthropic.Messages.prototype, 'create');
      }

      this._wrap(
        moduleExports.Anthropic.Messages.prototype,
        'create',
        AnthropicWrapper._patchMessageCreate(this.tracer)
      );
    } catch (e) {
      console.error('Error in _patch method:', e);
    }
  }

  protected _unpatch(moduleExports: typeof Anthropic) {
    this._unwrap(moduleExports.Anthropic.Messages.prototype, 'create');
  }
}
