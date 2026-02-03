import { registerInstrumentations } from '@opentelemetry/instrumentation';
import { InstrumentationType, OpenlitInstrumentations } from '../types';

import { TracerProvider } from '@opentelemetry/api';
import OpenAIInstrumentation from './openai';
import AnthropicInstrumentation from './anthropic';
import CohereInstrumentation from './cohere';

export default class Instrumentations {
  static availableInstrumentations: OpenlitInstrumentations = {
    openai: new OpenAIInstrumentation(),
    anthropic: new AnthropicInstrumentation(),
    cohere: new CohereInstrumentation(),
  };

  static setup(
    tracerProvider: TracerProvider,
    disabledInstrumentations: string[] = [],
    instrumentations?: OpenlitInstrumentations
  ) {
    if (instrumentations === undefined) {
      const filteredInstrumentations = this.getFilteredInstrumentations(disabledInstrumentations);
      registerInstrumentations({
        instrumentations: filteredInstrumentations.map(([_, instrumentation]) => instrumentation),
        tracerProvider,
      });
    } else {
      const filteredInstrumentations = this.getFilteredInstrumentations(
        disabledInstrumentations,
        instrumentations
      );
      filteredInstrumentations.forEach(([k, instrumentation]) => {
        if (this.availableInstrumentations[k].setTracerProvider) {
          this.availableInstrumentations[k].setTracerProvider(tracerProvider);
        }
        if (this.availableInstrumentations[k].manualPatch) {
          this.availableInstrumentations[k].manualPatch(instrumentation);
        }
      });
      registerInstrumentations({
        tracerProvider,
      });
    }
  }

  static getFilteredInstrumentations(
    disabledInstrumentations: string[],
    instrumentations?: OpenlitInstrumentations
  ): [InstrumentationType, any][] {
    const availableInstrumentations = instrumentations || this.availableInstrumentations;
    return Object.keys(availableInstrumentations)
      .filter((k) => {
        if (disabledInstrumentations.includes(k)) {
          if (typeof availableInstrumentations[k as InstrumentationType].disable === 'function') {
            availableInstrumentations[k as InstrumentationType].disable();
          }
          return false;
        }

        if (typeof availableInstrumentations[k as InstrumentationType].enable === 'function') {
          availableInstrumentations[k as InstrumentationType].enable();
        }

        return true;
      })
      .map((k) => [k as InstrumentationType, availableInstrumentations[k as InstrumentationType]]);
  }
}
