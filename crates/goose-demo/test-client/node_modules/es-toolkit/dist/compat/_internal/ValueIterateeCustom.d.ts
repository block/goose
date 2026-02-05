import { IterateeShorthand } from './IterateeShorthand.js';

type ValueIterateeCustom<T, TResult> = ((value: T) => TResult) | IterateeShorthand<T>;

export type { ValueIterateeCustom };
